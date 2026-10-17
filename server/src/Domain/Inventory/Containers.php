<?php
declare(strict_types=1);

namespace Scutoons\Domain\Inventory;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Catalog\Catalog;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Ids;

/**
 * Containers – inventory entries that hold entries (agents/research/SCORES.md §2, @scutoons/content containers.ts).
 * A container is an item with `holds` (categories / item ids, `max`, an `overfill` that breaks it); an entry lies in a
 * container entry of the same account (`inventory.container_id`). The reader moves things by hand (in / out); an
 * overfilled container remembers since when (`overfilled_at`) and breaks into `overfill.breaks_into` after
 * `overfill.days` – settled whenever the inventory is read (no timer). Taking out back to `max` in time saves it.
 */
final class Containers
{
    private const DAY = 86400;

    public function __construct(
        private readonly Database $db,
        private readonly Clock $clock,
        private readonly Catalog $catalog,
    ) {
    }

    /**
     * Moves an entry into a container, or out (`into` null) – { entry_id, into, overfill?, qty? }. `qty` = how many of a
     * stack (none = all of it); never more than the container takes – the rest stays where it was (research/PACKING.md
     * §2c, owner 2026-10-16: "never more than fits"). A stackable merges into the same item's stack where it lands.
     * 404 unknown_entry, 400 invalid_qty, 409 not_a_container / not_set_up / does_not_fit / container_in_container / worn
     * / container_full, 409 overfill_warning { max, days } until the client sends `overfill: true` (the reader agreed).
     * @param array<string, mixed> $body
     */
    public function move(string $userId, array $body): void
    {
        $entryId = is_string($body['entry_id'] ?? null) ? $body['entry_id'] : '';
        $into = $body['into'] ?? null;
        if ($into !== null && !is_string($into)) throw new ApiError(400, 'invalid_into', 'into is a container entry id or null.');
        $qty = $body['qty'] ?? null;
        if ($qty !== null && (!is_int($qty) || $qty < 1)) throw new ApiError(400, 'invalid_qty', 'qty is a whole number ≥ 1.');
        $this->db->transaction(function () use ($userId, $entryId, $into, $body, $qty) {
            $this->settle($userId);
            $entry = $this->entry($userId, $entryId);
            $from = is_string($entry['container_id'] ?? null) ? $entry['container_id'] : null;
            if ($into === $from) return;
            $n = min(is_int($qty) ? $qty : (int) $entry['qty'], (int) $entry['qty']);
            if ($into !== null) $n = $this->putIn($userId, $entry, $into, ($body['overfill'] ?? false) === true, $n);
            $this->shift($entry, $into, $n);
            if ($into !== null) $this->recount($userId, $into);
            if ($from !== null) $this->recount($userId, $from);
        });
    }

    /**
     * Puts two entries of the same kind onto one field (owner 2026-10-17: two cucumber crosses "auf einem Feld", like
     * Stardew – by hand) – { entry_id, onto }: `entry_id`'s pieces join `onto`, the entry goes. The same item and variant
     * at the same place (loose or in the same container); never a container, standing furniture, an ageing item or a piece
     * of its own (a code, an edition number – @scutoons/content `canStack`). 400 same_entry · 404 unknown_entry ·
     * 409 not_same / own_piece / worn.
     * @param array<string, mixed> $body
     */
    public function stack(string $userId, array $body): void
    {
        $entryId = is_string($body['entry_id'] ?? null) ? $body['entry_id'] : '';
        $ontoId = is_string($body['onto'] ?? null) ? $body['onto'] : '';
        if ($entryId === $ontoId) throw new ApiError(400, 'same_entry', 'Pick two pieces.');
        $this->db->transaction(function () use ($userId, $entryId, $ontoId) {
            $entry = $this->entry($userId, $entryId);
            $onto = $this->entry($userId, $ontoId);
            $item = $this->catalog->item((string) $entry['item_id']) ?? [];
            if ($entry['item_id'] !== $onto['item_id'] || $entry['variant'] !== $onto['variant'] || $entry['container_id'] !== $onto['container_id']) {
                throw new ApiError(409, 'not_same', 'Only two of the same kind go onto one field.');
            }
            if (($item['holds'] ?? null) !== null || ($item['ages'] ?? null) !== null || $entry['home_id'] !== null || $onto['home_id'] !== null) {
                throw new ApiError(409, 'not_same', 'This kind stays one piece per field.');
            }
            foreach ([$entry, $onto] as $row) {
                if ($row['code'] !== null || $row['edition_number'] !== null) throw new ApiError(409, 'own_piece', 'A piece with its own code or number stays on its own.');
            }
            if ($this->db->value('SELECT 1 FROM avatar_slots WHERE user_id = ? AND entry_id = ?', [$userId, $entry['id']])) {
                throw new ApiError(409, 'worn', 'Take it off first.');
            }
            $this->db->run('UPDATE inventory SET qty = qty + ?, updated_at = ? WHERE id = ?', [(int) $entry['qty'], $this->clock->utc(), $onto['id']]);
            $this->db->run('DELETE FROM inventory WHERE id = ?', [$entry['id']]);
        });
    }

    /**
     * The checks for moving `$n` of an entry into a container; returns how many go in (capped by the room left).
     * @param array<string, mixed> $entry
     */
    private function putIn(string $userId, array $entry, string $into, bool $overfill, int $n): int
    {
        $container = $this->entry($userId, $into);
        $holds = $this->holds((string) $container['item_id']);
        if ($holds === null) throw new ApiError(409, 'not_a_container', 'That is not a container.');
        // furniture (a shelf, a CD tower) is filled where it is set up – a home; a carried container (a vinyl bag)
        // anywhere (owner 2026-10-16: "furniture can only be stocked when setup in a place")
        if (($this->catalog->item((string) $container['item_id'])['furniture'] ?? null) !== null && ($container['home_id'] ?? null) === null) {
            throw new ApiError(409, 'not_set_up', 'Set it up in your home first – furniture is filled where it stands.');
        }
        $item = $this->catalog->item((string) $entry['item_id']) ?? [];
        if (($item['holds'] ?? null) !== null) throw new ApiError(409, 'container_in_container', 'A container cannot go into a container.');
        if (($entry['home_id'] ?? null) !== null) throw new ApiError(409, 'furniture_stays', 'It stands in your home – furniture stays there.');
        if (!self::fits($holds, $item, (string) $entry['item_id'])) throw new ApiError(409, 'does_not_fit', 'This does not fit in there.');
        if ($this->db->value('SELECT 1 FROM avatar_slots WHERE user_id = ? AND entry_id = ?', [$userId, $entry['id']])) {
            throw new ApiError(409, 'worn', 'Take it off first.');
        }
        $content = $this->content($into);
        $max = is_int($holds['max'] ?? null) ? $holds['max'] : null;
        if ($max === null) return $n;
        // what fits without overfilling goes in, the rest stays
        if ($content < $max) return min($n, $max - $content);
        $over = is_array($holds['overfill'] ?? null) ? $holds['overfill'] : null;
        if ($over === null || $content >= (int) $over['max']) throw new ApiError(409, 'container_full', 'It is full.', ['max' => $over['max'] ?? $max]);
        if (!$overfill) throw new ApiError(409, 'overfill_warning', 'Overfilled, it falls apart.', ['max' => $max, 'days' => (int) $over['days']]);
        return min($n, (int) $over['max'] - $content);
    }

    /**
     * Moves `$n` pieces of an entry to `$into` (null = loose): a stackable joins the same item's stack there (same
     * variant, no code), a part of a stack becomes its own entry; the rest stays.
     * @param array<string, mixed> $entry
     */
    private function shift(array $entry, ?string $into, int $n): void
    {
        $now = $this->clock->utc();
        $all = $n >= (int) $entry['qty'];
        $stackable = !empty($this->catalog->item((string) $entry['item_id'])['stackable']);
        $target = $stackable && $entry['code'] === null ? $this->db->value(
            'SELECT id FROM inventory WHERE user_id = ? AND item_id = ? AND variant <=> ? AND code IS NULL AND home_id IS NULL AND qty > 0 AND id <> ? AND '
                . ($into === null ? 'container_id IS NULL' : 'container_id = ?') . ' ORDER BY acquired_at, id LIMIT 1',
            array_merge([$entry['user_id'], $entry['item_id'], $entry['variant'], $entry['id']], $into === null ? [] : [$into])
        ) : null;
        if (is_string($target)) {
            $this->db->run('UPDATE inventory SET qty = qty + ?, updated_at = ? WHERE id = ?', [$n, $now, $target]);
            if ($all) $this->db->run('DELETE FROM inventory WHERE id = ?', [$entry['id']]);
            else $this->db->run('UPDATE inventory SET qty = qty - ?, updated_at = ? WHERE id = ?', [$n, $now, $entry['id']]);
        } elseif ($all) {
            $this->db->run('UPDATE inventory SET container_id = ?, updated_at = ? WHERE id = ?', [$into, $now, $entry['id']]);
        } else {
            $this->db->run('INSERT INTO inventory (id, user_id, item_id, variant, qty, source, code, edition_number, container_id, aged_since, acquired_at, updated_at) '
                . 'SELECT ?, user_id, item_id, variant, ?, source, code, edition_number, ?, aged_since, acquired_at, ? FROM inventory WHERE id = ?', [Ids::uuid(), $n, $into, $now, $entry['id']]);
            $this->db->run('UPDATE inventory SET qty = qty - ?, updated_at = ? WHERE id = ?', [$n, $now, $entry['id']]);
        }
    }

    /**
     * Does an item fit a container's rule: its category, its id or its sound-carrier format is listed
     * (@scutoons/content `fitsIn`)?
     * @param array<string, mixed> $holds
     * @param array<string, mixed> $item
     */
    private static function fits(array $holds, array $item, string $itemId): bool
    {
        return in_array($item['category'] ?? null, (array) ($holds['categories'] ?? []), true)
            || in_array($itemId, (array) ($holds['items'] ?? []), true)
            || (is_string($item['format'] ?? null) && in_array($item['format'], (array) ($holds['formats'] ?? []), true));
    }

    /** Sets / clears a container's `overfilled_at` from what it holds now (back to `max` = saved). */
    public function recount(string $userId, string $containerId): void
    {
        $container = $this->db->one('SELECT id, item_id, overfilled_at FROM inventory WHERE id = ? AND user_id = ?', [$containerId, $userId]);
        if (!$container) return;
        $max = $this->holds((string) $container['item_id'])['max'] ?? null;
        $over = is_int($max) && $this->content($containerId) > $max;
        if ($over && $container['overfilled_at'] === null) $this->db->run('UPDATE inventory SET overfilled_at = ? WHERE id = ?', [$this->clock->utc(), $containerId]);
        elseif (!$over && $container['overfilled_at'] !== null) $this->db->run('UPDATE inventory SET overfilled_at = NULL WHERE id = ?', [$containerId]);
    }

    /**
     * Breaks the containers that stayed overfilled past their days: the contents fall out loose, the entry becomes
     * `breaks_into` (the same variant and age). Runs before every read of the inventory (InventoryService::entries).
     */
    public function settle(string $userId): void
    {
        $rows = $this->db->all('SELECT id, item_id, overfilled_at FROM inventory WHERE user_id = ? AND overfilled_at IS NOT NULL', [$userId]);
        $now = $this->clock->time();
        foreach ($rows as $row) {
            $over = $this->holds((string) $row['item_id'])['overfill'] ?? null;
            if (!is_array($over)) continue;
            if (strtotime($row['overfilled_at'] . ' UTC') + (int) $over['days'] * self::DAY > $now) continue;
            $at = $this->clock->utc();
            $this->db->run('UPDATE inventory SET container_id = NULL, updated_at = ? WHERE container_id = ?', [$at, $row['id']]);
            $this->db->run('UPDATE inventory SET item_id = ?, overfilled_at = NULL, aged_since = COALESCE(aged_since, acquired_at), updated_at = ? WHERE id = ?', [$over['breaks_into'], $at, $row['id']]);
        }
    }

    /**
     * Puts entries into `$toId` (a recipe turned one container into another – the colour, the upgrade: the scores stay
     * in the book). What doesn't fit the new container's rule lies loose; its capacity counts afterwards (recount).
     * @param list<string> $entryIds
     */
    public function carryInto(string $userId, array $entryIds, string $toId): void
    {
        $holds = $this->holds((string) $this->db->value('SELECT item_id FROM inventory WHERE id = ?', [$toId]));
        foreach ($entryIds as $id) {
            $row = $this->db->one('SELECT item_id FROM inventory WHERE id = ? AND user_id = ?', [$id, $userId]);
            if (!$row) continue;
            $item = $this->catalog->item((string) $row['item_id']) ?? [];
            $fits = $holds !== null && self::fits($holds, $item, (string) $row['item_id']);
            $this->db->run('UPDATE inventory SET container_id = ? WHERE id = ?', [$fits ? $toId : null, $id]);
        }
        $this->recount($userId, $toId);
    }

    /**
     * The entries lying in a container
     * @return list<string>
     */
    public function contentIds(string $containerId): array
    {
        return array_map(fn($r) => (string) $r['id'], $this->db->all('SELECT id FROM inventory WHERE container_id = ? AND qty > 0', [$containerId]));
    }

    private function content(string $containerId): int
    {
        return (int) $this->db->value('SELECT COALESCE(SUM(qty), 0) FROM inventory WHERE container_id = ? AND qty > 0', [$containerId]);
    }

    /** @return array<string, mixed>|null */
    private function holds(string $itemId): ?array
    {
        $holds = $this->catalog->item($itemId)['holds'] ?? null;
        return is_array($holds) ? $holds : null;
    }

    /** @return array<string, mixed> */
    private function entry(string $userId, string $entryId): array
    {
        return $this->db->one('SELECT * FROM inventory WHERE id = ? AND user_id = ? AND qty > 0', [$entryId, $userId])
            ?? throw new ApiError(404, 'unknown_entry', 'Unknown inventory entry.');
    }
}
