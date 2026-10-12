<?php
declare(strict_types=1);

namespace Scutoons\Domain\Reader;

use PDOException;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Catalog\Catalog;
use Scutoons\Domain\Inventory\InventoryService;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Time;

/**
 * The reader's stats and effects (agents/ITEMS.md "Substances"; same math as substances.ts in @scutoons/content).
 *
 *   stats    catalog `stats`: { <id>: { label, baseline 0–100, half_life_days } } (testo, appetite). member_stats stores
 *            the level at updated_at; the level now = baseline + (level − baseline) · 0.5^(days / half_life_days) – lazily
 *            on read, no cron. No row = the baseline.
 *   effects  member_effects: a temporary reader effect (psychedelic) running until `until`.
 *   use      POST /inventory/use/ { item_id }: one piece of a substance (an item with effects / effect and the `apply`
 *            skill) is consumed and applied in one transaction: every stat moves from its current level by `delta`
 *            (clamped 0–100), a timed effect runs until now + duration_min (replacing a running one of that kind).
 * Game mechanics only – no real-world claims in any text.
 */
final class ReaderState
{
    public const MIN = 0.0;
    public const MAX = 100.0;

    public function __construct(
        private readonly Database $db,
        private readonly Clock $clock,
        private readonly Catalog $catalog,
        private readonly InventoryService $inventory,
    ) {
    }

    /** @return array<string, array<string, mixed>> the catalog's stat definitions */
    public function definitions(): array
    {
        $stats = $this->catalog->data()['stats'] ?? [];
        return is_array($stats) ? $stats : [];
    }

    /** The level now from a stored { level, updated_at } (or the baseline): decayed toward the baseline. */
    public static function level(float $baseline, float $halfLifeDays, ?float $stored, ?int $storedAt, int $now): float
    {
        if ($stored === null || $storedAt === null) return $baseline;
        $days = max(0, $now - $storedAt) / 86400;
        $factor = $halfLifeDays > 0 ? 0.5 ** ($days / $halfLifeDays) : 0.0;
        return self::clamp($baseline + ($stored - $baseline) * $factor);
    }

    public static function clamp(float $v): float
    {
        return min(self::MAX, max(self::MIN, $v));
    }

    /**
     * GET /me/ stats: every catalog stat → { level, updated_at }: the level as of updated_at = now (decayed, rounded to
     * 0.1), so a client can keep decaying it from there (statLevel in @scutoons/content).
     * @return array<string, array{level: float, updated_at: string|null}>
     */
    public function stats(string $userId): array
    {
        $rows = [];
        try {
            foreach ($this->db->all('SELECT stat, level, updated_at FROM member_stats WHERE user_id = ?', [$userId]) as $r) $rows[(string) $r['stat']] = $r;
        } catch (PDOException) {
            // not migrated yet: baselines only
        }
        $out = [];
        foreach ($this->definitions() as $id => $def) {
            $row = $rows[$id] ?? null;
            $at = $row ? strtotime($row['updated_at'] . ' UTC') : null;
            $level = self::level((float) ($def['baseline'] ?? 50), (float) ($def['half_life_days'] ?? 1), $row ? (float) $row['level'] : null, $at === false ? null : $at, $this->clock->time());
            $out[(string) $id] = ['level' => round($level, 1), 'updated_at' => gmdate('c', $this->clock->time())];
        }
        return $out;
    }

    /**
     * GET /me/ effects: the running ones [{ kind, until (ISO) }].
     * @return list<array{kind: string, until: string|null}>
     */
    public function effects(string $userId): array
    {
        try {
            $rows = $this->db->all('SELECT kind, until FROM member_effects WHERE user_id = ? AND until > ? ORDER BY until', [$userId, $this->clock->utc()]);
        } catch (PDOException) {
            return [];
        }
        return array_map(fn(array $r) => ['kind' => (string) $r['kind'], 'until' => Time::iso($r['until'])], $rows);
    }

    /**
     * POST /inventory/use/ { item_id } – consume one piece and apply it (one transaction). 404 unknown_item,
     * 422 not_usable (no `apply` skill / no effects), 422 missing (none owned).
     * @param array<string, mixed> $body
     * @return array<string, mixed> the item
     */
    public function use(string $userId, array $body): array
    {
        $itemId = is_string($body['item_id'] ?? null) ? $body['item_id'] : '';
        $item = $this->catalog->item($itemId) ?? throw new ApiError(404, 'unknown_item', 'Unknown item.');
        $applies = array_filter($item['skills'] ?? [], fn($s) => is_array($s) && ($s['type'] ?? null) === 'apply') !== [];
        if (!$applies || (empty($item['effects']) && empty($item['effect']) && empty($item['sober']))) throw new ApiError(422, 'not_usable', 'This item cannot be used like that.');
        $this->db->transaction(function () use ($userId, $item) {
            if ($this->inventory->consumable($userId, (string) $item['id']) < 1) throw new ApiError(422, 'missing', 'This item is not in your inventory.', ['item_id' => $item['id']]);
            $this->inventory->remove($userId, (string) $item['id'], 1);
            $now = $this->clock->time();
            $at = $this->clock->utc();
            $defs = $this->definitions();
            foreach ($item['effects'] ?? [] as $e) {
                $def = $defs[$e['stat'] ?? ''] ?? null;
                if (!is_array($def)) continue;
                $row = $this->db->one('SELECT level, updated_at FROM member_stats WHERE user_id = ? AND stat = ?', [$userId, $e['stat']]);
                $storedAt = $row ? strtotime($row['updated_at'] . ' UTC') : null;
                $current = self::level((float) $def['baseline'], (float) $def['half_life_days'], $row ? (float) $row['level'] : null, $storedAt === false ? null : $storedAt, $now);
                $next = self::clamp($current + (float) $e['delta']);
                $this->db->run('DELETE FROM member_stats WHERE user_id = ? AND stat = ?', [$userId, $e['stat']]);
                $this->db->run('INSERT INTO member_stats (user_id, stat, level, updated_at) VALUES (?, ?, ?, ?)', [$userId, $e['stat'], $next, $at]);
            }
            $this->db->run('DELETE FROM member_effects WHERE user_id = ? AND until <= ?', [$userId, $at]);
            // sobering up (catalog `sober`, owner 2026-10-08): every running effect loses that share of its remaining time
            $sober = (float) ($item['sober'] ?? 0);
            if ($sober > 0 && $sober < 1) {
                foreach ($this->db->all('SELECT kind, until FROM member_effects WHERE user_id = ?', [$userId]) as $row) {
                    $left = max(0, (int) strtotime($row['until'] . ' UTC') - $now);
                    $this->db->run('UPDATE member_effects SET until = ? WHERE user_id = ? AND kind = ?', [$this->clock->utc((int) round($left * (1 - $sober))), $userId, $row['kind']]);
                }
            }
            if (is_array($item['effect'] ?? null)) {
                $kind = (string) $item['effect']['kind'];
                $this->db->run('DELETE FROM member_effects WHERE user_id = ? AND kind = ?', [$userId, $kind]);
                $this->db->run('INSERT INTO member_effects (user_id, kind, until) VALUES (?, ?, ?)', [$userId, $kind, $this->clock->utc((int) round(((float) $item['effect']['duration_min']) * 60))]);
            }
        });
        return $item;
    }

    /** POST /effects/end/ { kind } – ends a running effect ("Effekt beenden"); true when one was running. */
    public function end(string $userId, string $kind): bool
    {
        return $this->db->run('DELETE FROM member_effects WHERE user_id = ? AND kind = ?', [$userId, $kind])->rowCount() > 0;
    }
}
