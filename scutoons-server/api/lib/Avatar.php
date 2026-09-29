<?php
declare(strict_types=1);

/** The reader's avatar: one inventory entry per body-part slot (catalog `slots`, item `slot`) */
final class Avatar
{
    /** @return array{slots: array<string, ?string>} every catalog slot, entry id or null */
    public static function toJson(string $userId): array
    {
        $slots = array_fill_keys(Catalog::slots(), null);
        foreach (Db::all('SELECT slot, entry_id FROM avatar_slots WHERE user_id = ?', [$userId]) as $row) {
            if (array_key_exists($row['slot'], $slots)) $slots[$row['slot']] = $row['entry_id'];
        }
        return ['slots' => (object) $slots];
    }

    /** GET /avatar/ → { avatar } */
    public static function get(): never
    {
        $user = Auth::user();
        Http::json(200, ['status' => 'success', 'avatar' => self::toJson($user['id'])]);
    }

    /**
     * PUT /avatar/ { slots: { <slot>: entryId|null } } → { status, avatar } (only the given slots change)
     * 422 invalid_slot (unknown slot), not_owned (entry is not the user's), wrong_slot (item is not worn there)
     */
    public static function put(array $body): never
    {
        $user = Auth::user();
        $slots = $body['slots'] ?? null;
        if (!is_array($slots) || ($slots !== [] && array_is_list($slots))) {
            throw new ApiError(400, 'invalid_slots', 'slots must be an object { slot: entryId|null }.');
        }
        $known = Catalog::slots();
        $changes = [];
        foreach ($slots as $slot => $entryId) {
            $slot = (string) $slot;
            if (!in_array($slot, $known, true)) throw new ApiError(422, 'invalid_slot', "Unknown slot $slot.", ['slot' => $slot]);
            if ($entryId === null) { $changes[$slot] = null; continue; }
            if (!is_string($entryId)) throw new ApiError(422, 'not_owned', 'Unknown inventory entry.', ['slot' => $slot]);
            $entry = Db::one('SELECT * FROM inventory WHERE id = ? AND user_id = ? AND qty > 0', [$entryId, $user['id']]);
            if (!$entry) throw new ApiError(422, 'not_owned', 'This item is not in your inventory.', ['slot' => $slot]);
            $item = Catalog::item($entry['item_id']);
            if (!$item || ($item['slot'] ?? null) !== $slot) {
                throw new ApiError(422, 'wrong_slot', 'This item cannot be worn there.', ['slot' => $slot, 'item_slot' => $item['slot'] ?? null]);
            }
            $changes[$slot] = $entryId;
        }
        Db::transaction(function () use ($user, $changes) {
            foreach ($changes as $slot => $entryId) {
                Db::run('DELETE FROM avatar_slots WHERE user_id = ? AND slot = ?', [$user['id'], $slot]);
                if ($entryId !== null) {
                    // One entry can only be worn once
                    Db::run('DELETE FROM avatar_slots WHERE user_id = ? AND entry_id = ?', [$user['id'], $entryId]);
                    Db::run('INSERT INTO avatar_slots (user_id, slot, entry_id, updated_at) VALUES (?, ?, ?, ?)', [$user['id'], $slot, $entryId, Db::now()]);
                }
            }
        });
        Http::json(200, ['status' => 'success', 'avatar' => self::toJson($user['id'])]);
    }
}
