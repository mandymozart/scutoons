<?php
declare(strict_types=1);

namespace Scutoons\Domain\Catalog;

use PDOException;
use Scutoons\Domain\ApiError;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Time;

/**
 * When a picked-up placed item is back on its slice (agents/ITEMS.md "Placed items" → "Respawn"; the client mirror is
 * @scutoons/content respawn.ts). The ITEM decides: `respawn: { default, min?, max? }` in config/items.json – an interval
 * is "visit" (every visit / reload) or { minutes | hours | days: n } – and none / null = it never reappears. Admin →
 * Catalog may override an item's rule: the runtime setting `item_respawn` (table `settings`, JSON map item id → rule or
 * null; a missing key = the file's rule).
 *
 * A story grant of a respawning placement sends `respawn` (the placement's interval) and, for "visit", `visit` (the
 * client's visit id). The server checks the interval against the item's range (shorter than `min` → 422) and builds the
 * ref's period itself: `@m<minutes>:<time bucket>` from its own clock, `@v:<visit id>` for visits. The grant ref is
 * granted once (InventoryService), so a placement gives one piece per period; max counts stay authoritative.
 */
final class ItemRespawn
{
    public const SETTING = 'item_respawn';
    public const MAX_MINUTES = 365 * 1440;
    private const UNITS = ['minutes' => 1, 'hours' => 60, 'days' => 1440];
    private const JSON_FLAGS = JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE;

    /** @var array{rules: array<string, mixed>, updated_at: ?string, updated_by: ?string}|null|false */
    private array|null|false $stored = false;

    public function __construct(private readonly Database $db, private readonly Catalog $catalog, private readonly Clock $clock)
    {
    }

    /** An interval's length in minutes (visit = 0), null when it isn't one */
    public static function minutes(mixed $interval): ?int
    {
        if ($interval === 'visit') return 0;
        if (!is_array($interval) || count($interval) !== 1) return null;
        $unit = (string) array_key_first($interval);
        $n = $interval[$unit];
        if (!isset(self::UNITS[$unit]) || !is_int($n) || $n < 1) return null;
        $minutes = $n * self::UNITS[$unit];
        return $minutes <= self::MAX_MINUTES ? $minutes : null;
    }

    /**
     * Problems of a rule (null = never is fine)
     * @return list<string>
     */
    public static function problems(mixed $rule): array
    {
        if ($rule === null) return [];
        if (!is_array($rule) || array_is_list($rule)) return ['respawn must be { default, min?, max? } or null (never)'];
        $out = [];
        $default = self::minutes($rule['default'] ?? null);
        if ($default === null) $out[] = 'respawn.default must be "visit" or { minutes | hours | days: n }';
        $min = array_key_exists('min', $rule) ? self::minutes($rule['min']) : $default;
        $max = array_key_exists('max', $rule) ? self::minutes($rule['max']) : self::MAX_MINUTES;
        if (array_key_exists('min', $rule) && $min === null) $out[] = 'respawn.min must be "visit" or { minutes | hours | days: n }';
        if (array_key_exists('max', $rule) && $max === null) $out[] = 'respawn.max must be "visit" or { minutes | hours | days: n }';
        if ($default !== null && $min !== null && $max !== null && !($min <= $default && $default <= $max)) $out[] = 'respawn: min ≤ default ≤ max';
        foreach (array_keys($rule) as $key) {
            if (!in_array($key, ['default', 'min', 'max'], true)) $out[] = "respawn.$key is unknown";
        }
        return $out;
    }

    /**
     * The shortest / longest interval (minutes) a placement may choose
     * @param array<string, mixed> $rule
     * @return array{min: int, max: int}
     */
    public static function range(array $rule): array
    {
        $default = self::minutes($rule['default'] ?? null) ?? 0;
        return [
            'min' => array_key_exists('min', $rule) ? (self::minutes($rule['min']) ?? $default) : $default,
            'max' => array_key_exists('max', $rule) ? (self::minutes($rule['max']) ?? self::MAX_MINUTES) : self::MAX_MINUTES,
        ];
    }

    /**
     * An item's effective rule: the admin override, else config/items.json; null = never reappears
     * @return array<string, mixed>|null
     */
    public function rule(string $itemId): ?array
    {
        $rules = $this->stored()['rules'] ?? [];
        if (array_key_exists($itemId, $rules)) return is_array($rules[$itemId]) ? $rules[$itemId] : null;
        $rule = $this->catalog->item($itemId)['respawn'] ?? null;
        return is_array($rule) && self::problems($rule) === [] ? $rule : null;
    }

    /**
     * The catalog as served (GET /api/items/, the admin catalog) with the effective rules
     * @param array<string, mixed> $data
     * @return array<string, mixed>
     */
    public function apply(array $data): array
    {
        $rules = $this->stored()['rules'] ?? [];
        if (!$rules) return $data;
        foreach ($data['items'] as $i => $item) {
            if (is_array($item) && array_key_exists($item['id'] ?? '', $rules)) $data['items'][$i]['respawn'] = $rules[$item['id']];
        }
        return $data;
    }

    /**
     * The grant ref of a respawning placement: the placement's ref + the period. 422 respawn_not_allowed (the item never
     * reappears), respawn_out_of_range (shorter / longer than the item allows), 400 invalid_respawn / visit_required.
     * @param array<string, mixed> $item
     */
    public function periodRef(array $item, string $ref, mixed $interval, mixed $visit): string
    {
        $minutes = self::minutes($interval);
        if ($minutes === null) throw new ApiError(400, 'invalid_respawn', 'respawn must be "visit" or { minutes | hours | days: n }.');
        $rule = $this->rule((string) $item['id']);
        if ($rule === null) throw new ApiError(422, 'respawn_not_allowed', 'This item does not reappear.');
        $range = self::range($rule);
        if ($minutes < $range['min'] || $minutes > $range['max']) {
            throw new ApiError(422, 'respawn_out_of_range', 'This item cannot reappear that often.', ['min' => $range['min'], 'max' => $range['max']]);
        }
        if ($minutes === 0) {
            if (!is_string($visit) || !preg_match('/^[A-Za-z0-9_-]{4,32}$/', $visit)) throw new ApiError(400, 'visit_required', 'A respawn per visit needs the visit id.');
            $period = "v:$visit";
        } else {
            $period = 'm' . $minutes . ':' . intdiv($this->clock->time(), $minutes * 60);
        }
        return substr($ref, 0, 191 - strlen($period) - 1) . '@' . $period;
    }

    /**
     * The admin view: every item with its file rule, the override (if any) and the effective rule
     * @return array{items: list<array{id: string, file: mixed, override: mixed, overridden: bool, effective: mixed}>, updated_at: ?string, updated_by: ?string}
     */
    public function state(): array
    {
        $stored = $this->stored();
        $rules = $stored['rules'] ?? [];
        $items = [];
        foreach ($this->catalog->items() as $id => $item) {
            $id = (string) $id;
            $items[] = [
                'id' => $id,
                'file' => $item['respawn'] ?? null,
                'override' => $rules[$id] ?? null,
                'overridden' => array_key_exists($id, $rules),
                'effective' => $this->rule($id),
            ];
        }
        return ['items' => $items, 'updated_at' => $stored['updated_at'] ?? null, 'updated_by' => $stored['updated_by'] ?? null];
    }

    /**
     * Sets an item's override: a rule, null (never reappears), or – $reset – back to config/items.json. Returns whether
     * anything changed. 404 unknown_item, 400 invalid_respawn (with the problems).
     */
    public function update(string $itemId, mixed $rule, bool $reset, string $by): bool
    {
        if ($this->catalog->item($itemId) === null) throw new ApiError(404, 'unknown_item', 'Unknown item.');
        if (!$reset && ($problems = self::problems($rule)) !== []) throw new ApiError(400, 'invalid_respawn', implode('; ', $problems), ['problems' => $problems]);
        $stored = $this->stored();
        $rules = $stored['rules'] ?? [];
        if ($reset) {
            if (!array_key_exists($itemId, $rules)) return false;
            unset($rules[$itemId]);
        } else {
            if (array_key_exists($itemId, $rules) && $rules[$itemId] === $rule) return false;
            $rules[$itemId] = $rule;
        }
        $params = [(string) json_encode((object) $rules, self::JSON_FLAGS), $this->clock->utc(), substr($by, 0, 255), self::SETTING];
        if ($stored !== null) $this->db->run('UPDATE settings SET `value` = ?, updated_at = ?, updated_by = ? WHERE `key` = ?', $params);
        else $this->db->run('INSERT INTO settings (`value`, updated_at, updated_by, `key`) VALUES (?, ?, ?, ?)', $params);
        $this->stored = false;
        return true;
    }

    /** @return array{rules: array<string, mixed>, updated_at: ?string, updated_by: ?string}|null */
    private function stored(): ?array
    {
        if ($this->stored !== false) return $this->stored;
        try {
            $row = $this->db->one('SELECT `value`, updated_at, updated_by FROM settings WHERE `key` = ?', [self::SETTING]);
        } catch (PDOException) {
            $row = null; // before the settings migration
        }
        $rules = $row ? json_decode((string) $row['value'], true) : null;
        return $this->stored = is_array($rules)
            ? ['rules' => $rules, 'updated_at' => Time::iso($row['updated_at'] ?? null), 'updated_by' => isset($row['updated_by']) ? (string) $row['updated_by'] : null]
            : null;
    }
}
