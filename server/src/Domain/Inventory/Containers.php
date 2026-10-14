<?php
declare(strict_types=1);

namespace Scutoons\Domain\Inventory;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Catalog\Catalog;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Database\Database;

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
     * Moves an entry into a container, or out (`into` null) – { entry_id, into, overfill? }. 404 unknown_entry,
     * 409 not_a_container / does_not_fit / container_in_container / worn / container_full, 409 overfill_warning
     * { max, days } until the client sends `overfill: true` (the reader agreed to the warning).
     * @param array<string, mixed> $body
     */
    public function move(string $userId, array $body): void
    {
        $entryId = is_string($body['entry_id'] ?? null) ? $body['entry_id'] : '';
        $into = $body['into'] ?? null;
        if ($into !== null && !is_string($into)) throw new ApiError(400, 'invalid_into', 'into is a container entry id or null.');
        $this->db->transaction(function () use ($userId, $entryId, $into, $body) {
            $this->settle($userId);
            $entry = $this->entry($userId, $entryId);
            $from = is_string($entry['container_id'] ?? null) ? $entry['container_id'] : null;
            if ($into === $from) return;
            if ($into !== null) $this->putIn($userId, $entry, $into, ($body['overfill'] ?? false) === true);
            else $this->db->run('UPDATE inventory SET container_id = NULL, updated_at = ? WHERE id = ?', [$this->clock->utc(), $entry['id']]);
            if ($from !== null) $this->recount($userId, $from);
        });
    }

    /**
     * @param array<string, mixed> $entry
     */
    private function putIn(string $userId, array $entry, string $into, bool $overfill): void
    {
        $container = $this->entry($userId, $into);
        $holds = $this->holds((string) $container['item_id']);
        if ($holds === null) throw new ApiError(409, 'not_a_container', 'That is not a container.');
        $item = $this->catalog->item((string) $entry['item_id']) ?? [];
        if (($item['holds'] ?? null) !== null) throw new ApiError(409, 'container_in_container', 'A container cannot go into a container.');
        if (!in_array($item['category'] ?? null, (array) ($holds['categories'] ?? []), true) && !in_array($entry['item_id'], (array) ($holds['items'] ?? []), true)) {
            throw new ApiError(409, 'does_not_fit', 'This does not fit in there.');
        }
        if ($this->db->value('SELECT 1 FROM avatar_slots WHERE user_id = ? AND entry_id = ?', [$userId, $entry['id']])) {
            throw new ApiError(409, 'worn', 'Take it off first.');
        }
        $after = $this->content($into) + (int) $entry['qty'];
        $max = is_int($holds['max'] ?? null) ? $holds['max'] : null;
        $over = is_array($holds['overfill'] ?? null) ? $holds['overfill'] : null;
        if ($max !== null && $after > $max) {
            if ($over === null || $after > (int) $over['max']) throw new ApiError(409, 'container_full', 'It is full.', ['max' => $over['max'] ?? $max]);
            if (!$overfill) throw new ApiError(409, 'overfill_warning', 'Overfilled, it falls apart.', ['max' => $max, 'days' => (int) $over['days']]);
        }
        $now = $this->clock->utc();
        $this->db->run('UPDATE inventory SET container_id = ?, updated_at = ? WHERE id = ?', [$into, $now, $entry['id']]);
        $this->recount($userId, $into);
    }

    /** Sets / clears a container's `overfilled_at` from what it holds now (back to `max` = saved). */
    private function recount(string $userId, string $containerId): void
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
            $fits = $holds !== null && (in_array($item['category'] ?? null, (array) ($holds['categories'] ?? []), true) || in_array($row['item_id'], (array) ($holds['items'] ?? []), true));
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
