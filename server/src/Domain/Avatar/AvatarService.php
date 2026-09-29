<?php
declare(strict_types=1);

namespace Scutoons\Domain\Avatar;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Catalog\Catalog;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Database\Database;

/** The reader's avatar: one inventory entry per body-part slot (catalog `slots`, item `slot`) */
final class AvatarService
{
    public function __construct(
        private readonly Database $db,
        private readonly Clock $clock,
        private readonly Catalog $catalog,
    ) {
    }

    /** @return array{slots: object} every catalog slot → entry id or null (a JSON object also when empty) */
    public function toJson(string $userId): array
    {
        $slots = array_fill_keys($this->catalog->slots(), null);
        foreach ($this->db->all('SELECT slot, entry_id FROM avatar_slots WHERE user_id = ?', [$userId]) as $row) {
            if (array_key_exists($row['slot'], $slots)) $slots[$row['slot']] = $row['entry_id'];
        }
        return ['slots' => (object) $slots];
    }

    /**
     * PUT /avatar/ { slots: { <slot>: entryId|null } } – only the given slots change.
     * 400 invalid_slots, 422 invalid_slot (unknown slot), not_owned (entry is not the user's), wrong_slot.
     * @param array<string, mixed> $body
     */
    public function update(string $userId, array $body): void
    {
        $slots = $body['slots'] ?? null;
        if (!is_array($slots) || ($slots !== [] && array_is_list($slots))) {
            throw new ApiError(400, 'invalid_slots', 'slots must be an object { slot: entryId|null }.');
        }
        $known = $this->catalog->slots();
        $changes = [];
        foreach ($slots as $slot => $entryId) {
            $slot = (string) $slot;
            if (!in_array($slot, $known, true)) throw new ApiError(422, 'invalid_slot', "Unknown slot $slot.", ['slot' => $slot]);
            if ($entryId === null) { $changes[$slot] = null; continue; }
            if (!is_string($entryId)) throw new ApiError(422, 'not_owned', 'Unknown inventory entry.', ['slot' => $slot]);
            $entry = $this->db->one('SELECT * FROM inventory WHERE id = ? AND user_id = ? AND qty > 0', [$entryId, $userId]);
            if (!$entry) throw new ApiError(422, 'not_owned', 'This item is not in your inventory.', ['slot' => $slot]);
            $item = $this->catalog->item((string) $entry['item_id']);
            if (!$item || ($item['slot'] ?? null) !== $slot) {
                throw new ApiError(422, 'wrong_slot', 'This item cannot be worn there.', ['slot' => $slot, 'item_slot' => $item['slot'] ?? null]);
            }
            $changes[$slot] = $entryId;
        }
        $this->db->transaction(function () use ($userId, $changes) {
            foreach ($changes as $slot => $entryId) {
                $this->db->run('DELETE FROM avatar_slots WHERE user_id = ? AND slot = ?', [$userId, $slot]);
                if ($entryId !== null) {
                    // One entry can only be worn once
                    $this->db->run('DELETE FROM avatar_slots WHERE user_id = ? AND entry_id = ?', [$userId, $entryId]);
                    $this->db->run('INSERT INTO avatar_slots (user_id, slot, entry_id, updated_at) VALUES (?, ?, ?, ?)', [$userId, $slot, $entryId, $this->clock->utc()]);
                }
            }
        });
    }
}
