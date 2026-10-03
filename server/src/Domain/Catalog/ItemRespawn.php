<?php
declare(strict_types=1);

namespace Scutoons\Domain\Catalog;

use Scutoons\Domain\ApiError;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Time;
use stdClass;

/**
 * When a picked-up placed item is back on its slice (agents/ITEMS.md "Placed items" → "Respawn"; the client mirror is
 * @scutoons/content respawn.ts). The ITEM decides: `respawn: { default, min?, max? }` in its catalog row – an interval
 * is "visit" (every visit / reload) or { minutes | hours | days: n } – and none / null = it never reappears. Admin →
 * Catalog changes an item's rule: an update of the item (catalog_history). The runtime setting `item_respawn` that
 * held the overrides until 2026-10-04 was folded into the rows by the migration 2026-10-04-catalog and is gone.
 * "file" in the admin view = the rule of the seed (config/items.json); reset = back to it.
 *
 * A story grant of a respawning placement sends `respawn` (the placement's interval) and, for "visit", `visit` (the
 * client's visit id). The server checks the interval against the item's range (shorter than `min` → 422) and builds the
 * ref's period itself: `@m<minutes>:<time bucket>` from its own clock, `@v:<visit id>` for visits. The grant ref is
 * granted once (InventoryService), so a placement gives one piece per period; max counts stay authoritative.
 */
final class ItemRespawn
{
    /** the retired setting (migration 2026-10-04-catalog folds it into the items) */
    public const SETTING = 'item_respawn';
    public const MAX_MINUTES = 365 * 1440;
    private const UNITS = ['minutes' => 1, 'hours' => 60, 'days' => 1440];

    public function __construct(
        private readonly Database $db,
        private readonly Catalog $catalog,
        private readonly Clock $clock,
        private readonly CatalogStore $store,
        private readonly CatalogSeed $seed,
        private readonly CatalogAdmin $admin,
    ) {
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
     * An item's rule (its catalog row); null = never reappears
     * @return array<string, mixed>|null
     */
    public function rule(string $itemId): ?array
    {
        $rule = $this->catalog->item($itemId)['respawn'] ?? null;
        return is_array($rule) && self::problems($rule) === [] ? $rule : null;
    }

    /**
     * The catalog as served – unchanged since the rules are part of the items (kept for the callers)
     * @param array<string, mixed> $data
     * @return array<string, mixed>
     */
    public function apply(array $data): array
    {
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
     * The admin view: every item with the seed's rule (`file`, config/items.json), its own rule when that differs
     * (`override`, `overridden`) and the effective rule; the last respawn change (history)
     * @return array{items: list<array{id: string, file: mixed, override: mixed, overridden: bool, effective: mixed}>, updated_at: ?string, updated_by: ?string}
     */
    public function state(): array
    {
        $items = [];
        foreach ($this->catalog->items() as $id => $item) {
            $id = (string) $id;
            $seed = $this->seed->entry('items', $id);
            $file = is_array($seed) ? ($seed['respawn'] ?? null) : null;
            $overridden = self::differs(is_array($seed) ? $seed : [], $item);
            $items[] = [
                'id' => $id,
                'file' => $file,
                'override' => $overridden ? ($item['respawn'] ?? null) : null,
                'overridden' => $overridden,
                'effective' => $this->rule($id),
            ];
        }
        try {
            $last = $this->db->one("SELECT changed_at, changed_by FROM catalog_history WHERE kind = 'items' AND note = 'respawn' ORDER BY id DESC LIMIT 1");
        } catch (\PDOException) {
            $last = null; // before the migration
        }
        return ['items' => $items, 'updated_at' => Time::iso($last['changed_at'] ?? null), 'updated_by' => isset($last['changed_by']) ? (string) $last['changed_by'] : null];
    }

    /**
     * Sets an item's rule: a rule, null (never reappears), or – $reset – back to the seed's (config/items.json). An
     * update of the item (validated, catalog_history, note "respawn"). Returns whether anything changed. 404
     * unknown_item, 400 invalid_respawn (with the problems).
     */
    public function update(string $itemId, mixed $rule, bool $reset, string $by): bool
    {
        if ($this->catalog->item($itemId) === null) throw new ApiError(404, 'unknown_item', 'Unknown item.');
        if (!$reset && ($problems = self::problems($rule)) !== []) throw new ApiError(400, 'invalid_respawn', implode('; ', $problems), ['problems' => $problems]);
        $row = $this->store->row('items', $itemId) ?? throw new ApiError(404, 'unknown_item', 'Unknown item.');
        $item = json_decode($row['data']);
        if (!$item instanceof stdClass) throw new ApiError(500, 'invalid_item', 'The stored item is invalid.');
        if ($reset) {
            $seed = json_decode((string) json_encode($this->seed->entry('items', $itemId)));
            if ($seed instanceof stdClass && property_exists($seed, 'respawn')) $item->respawn = $seed->respawn;
            else unset($item->respawn);
        } else {
            $item->respawn = json_decode((string) json_encode($rule));
        }
        return $this->admin->put('items', $itemId, $item, $by, null, 'respawn')['changed'];
    }

    /**
     * @param array<string, mixed> $seed
     * @param array<string, mixed> $item
     */
    private static function differs(array $seed, array $item): bool
    {
        return array_key_exists('respawn', $seed) !== array_key_exists('respawn', $item) || ($seed['respawn'] ?? null) != ($item['respawn'] ?? null);
    }
}
