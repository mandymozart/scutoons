<?php
declare(strict_types=1);

namespace Scutoons\Domain\Avatar;

use PDOException;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Catalog\Catalog;
use Scutoons\Domain\Characters\CharacterUnlocks;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Database\Database;

/**
 * The reader's avatar: the body (a character's A-pose look, agents/ITEMS.md "Reader avatar bodies & character unlocks")
 * and one inventory entry per body-part slot (catalog `slots`, item `slot`).
 */
final class AvatarService
{
    public function __construct(
        private readonly Database $db,
        private readonly Clock $clock,
        private readonly Catalog $catalog,
        private readonly CharacterUnlocks $unlocks,
    ) {
    }

    /**
     * { slots, body, unlocked }: every catalog slot → entry id or null (a JSON object also when empty); body = the chosen
     * { character, look } or null (= the reader shows the first unlocked default); unlocked = CharacterUnlocks::toJson
     * @return array{slots: object, body: array{character: string, look: string}|null, unlocked: list<array<string, mixed>>}
     */
    public function toJson(string $userId): array
    {
        $slots = array_fill_keys($this->catalog->slots(), null);
        foreach ($this->db->all('SELECT slot, entry_id FROM avatar_slots WHERE user_id = ?', [$userId]) as $row) {
            if (array_key_exists($row['slot'], $slots)) $slots[$row['slot']] = $row['entry_id'];
        }
        return ['slots' => (object) $slots, 'body' => $this->body($userId), 'unlocked' => $this->unlocks->toJson($userId)];
    }

    /** @return array{character: string, look: string}|null the chosen body while it is still unlocked */
    public function body(string $userId): ?array
    {
        try {
            $row = $this->db->one('SELECT character_id, look_id FROM avatar_body WHERE user_id = ?', [$userId]);
        } catch (PDOException) {
            return null; // not migrated yet
        }
        if (!$row || !$this->unlocks->isUnlocked($userId, (string) $row['character_id'], (string) $row['look_id'])) return null;
        return ['character' => (string) $row['character_id'], 'look' => (string) $row['look_id']];
    }

    /**
     * PUT /avatar/ { slots?: { <slot>: entryId|null }, body?: { character, look }|null } – only what is given changes
     * (at least one of them). body: must be unlocked for the member and have an A-pose image; null = back to the default.
     * 400 invalid_slots (neither given / not an object), invalid_body; 422 invalid_slot (unknown slot), not_owned (entry
     * is not the user's), wrong_slot, locked (body not unlocked), no_apose (no A-pose image for that look).
     * @param array<string, mixed> $body
     */
    public function update(string $userId, array $body): void
    {
        $hasBody = array_key_exists('body', $body);
        if ($hasBody) $this->checkBody($userId, $body['body']);
        if (!array_key_exists('slots', $body) && $hasBody) {
            $this->saveBody($userId, $body['body']);
            return;
        }
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
        $this->db->transaction(function () use ($userId, $changes, $hasBody, $body) {
            if ($hasBody) $this->saveBody($userId, $body['body']);
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

    private function checkBody(string $userId, mixed $value): void
    {
        if ($value === null) return;
        if (!is_array($value) || !is_string($value['character'] ?? null) || !is_string($value['look'] ?? null)) {
            throw new ApiError(400, 'invalid_body', 'body must be { character, look } or null.');
        }
        [$character, $look] = [$value['character'], $value['look']];
        if (!$this->unlocks->isUnlocked($userId, $character, $look)) {
            throw new ApiError(422, 'locked', 'This character or look is not unlocked for your account.', ['character' => $character, 'look' => $look]);
        }
        if (!$this->unlocks->hasApose($character, $look)) {
            throw new ApiError(422, 'no_apose', 'This character has no A-pose image for that look.', ['character' => $character, 'look' => $look]);
        }
    }

    private function saveBody(string $userId, mixed $value): void
    {
        $this->db->run('DELETE FROM avatar_body WHERE user_id = ?', [$userId]);
        if (is_array($value)) {
            $this->db->run('INSERT INTO avatar_body (user_id, character_id, look_id, updated_at) VALUES (?, ?, ?, ?)', [$userId, $value['character'], $value['look'], $this->clock->utc()]);
        }
    }
}
