<?php
declare(strict_types=1);

namespace Scutoons\Domain\Avatar;

use PDOException;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Catalog\Catalog;
use Scutoons\Domain\Characters\CharacterUnlocks;
use Scutoons\Domain\Inventory\InventoryService;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Database\Database;

/**
 * The reader's avatar: the body (a character's A-pose look, agents/ITEMS.md "Reader avatar bodies & character unlocks")
 * and the worn items by **slot instance** of that body (agents/ITEMS.md "Slots per avatar body"): the A-pose look's
 * `slots` in the character registry ([{ id, slot }], a duo has `torso-left` + `torso-right`), or – for a body without
 * slots – one instance per catalog slot type (id = type). `avatar_slots.slot` stores the instance id (for single
 * figures the same value as the type, so rows from before the slot instances stay valid as they are).
 *
 * Worn items follow the reader across bodies: on a body switch (and when reading rows that don't fit the current body)
 * an item stays on the instance with the same id, otherwise it goes to the first free instance of its slot type, else
 * it comes off (the same rule as `remapAvatarSlots` in @scutoons/content).
 *
 * The naked A-pose look was removed (owner 2026-10-05): a body stored with a retired look reads as the character's
 * default look (migration 2026-10-05-no-naked moves the rows).
 *
 * Wardrobe (agents/ITEMS.md "Wardrobe"): the default wearables (starter items with a slot: SCUTOONS T-shirt, swim
 * pants, shorts, sandals) are granted at signup and lazily for accounts older than them (ensureWardrobe); the basic
 * dress (`default_worn`: T-shirt, shorts, sandals – the swim pants instead of the shorts on a body without a shorts
 * overlay) is put on where nothing of that slot type is worn (wearBasicDress). Putting an item on an
 * occupied instance swaps the old one back into the inventory (it stays an entry, just not worn). Every slot may be
 * empty: the base image is the swim A-pose with the pants painted in.
 */
final class AvatarService
{
    public function __construct(
        private readonly Database $db,
        private readonly Clock $clock,
        private readonly Catalog $catalog,
        private readonly CharacterUnlocks $unlocks,
        private readonly InventoryService $inventory,
    ) {
    }

    /**
     * Grants (and wears, where the slot type is free) the default wearables the member owns none of yet – accounts made
     * before an item became a starter. Returns the granted item ids (usually none: one cheap check per call).
     * @return list<string>
     */
    public function ensureWardrobe(string $userId): array
    {
        if (!$this->inventory->missingStarterWearables($userId)) return [];
        /** @var list<string> */
        return $this->db->transaction(function () use ($userId): array {
            $granted = $this->inventory->grantStarterWearables($userId);
            $this->wearBasicDress($userId, $granted);
            return $granted;
        });
    }

    /**
     * The basic dress for a character (basicDress in @scutoons/content): per slot type of the starter wearables, the
     * `default_worn` one with an overlay for the character, else another of that type with one (the swim pants on a body
     * without a shorts overlay), else the `default_worn` one; slot types without a `default_worn` item are left out.
     * @return list<array<string, mixed>> catalog items
     */
    public function basicDress(?string $character): array
    {
        $starters = array_values(array_filter($this->catalog->starters(), fn(array $i) => is_string($i['slot'] ?? null)));
        $has = fn(array $i) => $character !== null && is_array($i['wear'] ?? null)
            && array_filter(array_keys($i['wear']), fn($k) => $k === $character || str_starts_with((string) $k, "$character/")) !== [];
        $out = [];
        foreach ($this->catalog->slots() as $slot) {
            $ofType = array_values(array_filter($starters, fn(array $i) => $i['slot'] === $slot));
            $preferred = array_values(array_filter($ofType, fn(array $i) => ($i['default_worn'] ?? false) === true));
            if (!$preferred) continue;
            $pick = null;
            foreach ([...$preferred, ...$ofType] as $i) if ($has($i)) { $pick = $i; break; }
            $out[] = $pick ?? $preferred[0];
        }
        return $out;
    }

    /**
     * Puts the basic dress of the member's current body (chosen, else the default) on for the slot types of the items
     * just granted, where nothing of that type is worn yet (an avatar_slots row by slot type: it lands on the first
     * instance of that type of any body). No own transaction (signup, ensureWardrobe).
     * @param list<string> $granted item ids
     */
    public function wearBasicDress(string $userId, array $granted): void
    {
        if (!$granted) return;
        $types = array_unique(array_filter(array_map(fn(string $id) => $this->catalog->item($id)['slot'] ?? null, $granted), 'is_string'));
        $body = $this->body($userId) ?? $this->unlocks->defaultBody($userId);
        $worn = array_map(fn(array $w) => $w['type'], $this->stored($userId));
        $now = $this->clock->utc();
        foreach ($this->basicDress($body['character'] ?? null) as $item) {
            if (!in_array($item['slot'], $types, true) || in_array($item['slot'], $worn, true)) continue;
            $entry = $this->db->value('SELECT id FROM inventory WHERE user_id = ? AND item_id = ? AND qty > 0 ORDER BY acquired_at, id LIMIT 1', [$userId, $item['id']]);
            if ($entry === null || $entry === false) continue;
            // DELETE only a row that exists: on MariaDB/MySQL (InnoDB, REPEATABLE READ) a DELETE that matches nothing
            // takes a gap lock, and two signups at the same moment (time-ordered user ids = the same gap) then
            // deadlock on their INSERTs (1213, found by parallel signups on MariaDB; WardrobeTest). A plain read takes no lock.
            if ($this->db->value('SELECT 1 FROM avatar_slots WHERE user_id = ? AND slot = ?', [$userId, $item['slot']]) !== null) {
                $this->db->run('DELETE FROM avatar_slots WHERE user_id = ? AND slot = ?', [$userId, $item['slot']]);
            }
            $this->db->run('INSERT INTO avatar_slots (user_id, slot, entry_id, updated_at) VALUES (?, ?, ?, ?)', [$userId, $item['slot'], (string) $entry, $now]);
            $worn[] = $item['slot'];
        }
    }

    /**
     * { slots, body, unlocked }: every slot instance of the current body → entry id or null (a JSON object also when
     * empty); body = the chosen { character, look } or null (= the reader shows the first unlocked default);
     * unlocked = CharacterUnlocks::toJson
     * @return array{slots: object, body: array{character: string, look: string}|null, unlocked: list<array<string, mixed>>}
     */
    public function toJson(string $userId): array
    {
        $body = $this->body($userId);
        $slots = $this->remap($this->stored($userId), $this->instances($body ?? $this->unlocks->defaultBody($userId)));
        return ['slots' => (object) $slots, 'body' => $body, 'unlocked' => $this->unlocks->toJson($userId)];
    }

    /**
     * The chosen body while it is still unlocked; a retired look (the removed `naked`) reads as the character's default
     * look.
     * @return array{character: string, look: string}|null
     */
    public function body(string $userId): ?array
    {
        $row = $this->storedBody($userId);
        if ($row && $this->unlocks->isRetired($row['character'], $row['look'])) {
            $look = $this->unlocks->defaultLook($row['character']);
            $row = $look !== null ? ['character' => $row['character'], 'look' => $look] : null;
        }
        if (!$row || !$this->unlocks->isUnlocked($userId, $row['character'], $row['look'])) return null;
        return $row;
    }

    /** @return array{character: string, look: string}|null the stored avatar_body row, unchecked */
    private function storedBody(string $userId): ?array
    {
        try {
            $row = $this->db->one('SELECT character_id, look_id FROM avatar_body WHERE user_id = ?', [$userId]);
        } catch (PDOException) {
            return null; // not migrated yet
        }
        return $row ? ['character' => (string) $row['character_id'], 'look' => (string) $row['look_id']] : null;
    }

    /**
     * The slot instances of a body (null or a body without slots: one per catalog slot type).
     * @param array{character: string, look: string}|null $body
     * @return list<array{id: string, slot: string}>
     */
    public function instances(?array $body): array
    {
        $out = $body ? $this->unlocks->slotInstances($body['character'], $body['look']) : [];
        if ($out) return $out;
        return array_map(fn(string $t) => ['id' => $t, 'slot' => $t], $this->catalog->slots());
    }

    /**
     * PUT /avatar/ { slots?: { <instance id>: entryId|null }, body?: { character, look }|null } – only what is given
     * changes (at least one of them). body: must be unlocked for the member and have an A-pose image; null = back to the
     * default; the worn items move to the new body (same instance id, else the first free instance of their type).
     * slots: instance ids of the (new) body; the item's slot type must match the instance's type.
     * 400 invalid_slots (neither given / not an object), invalid_body; 422 invalid_slot (no such instance on this body),
     * not_owned (entry is not the user's), wrong_slot, locked (body not unlocked), no_apose (no A-pose image for that look).
     * Swap: the entry that was on an instance just comes off (stays in the inventory).
     * @param array<string, mixed> $body
     */
    public function update(string $userId, array $body): void
    {
        $hasBody = array_key_exists('body', $body);
        if ($hasBody) $this->checkBody($userId, $body['body']);
        $slots = $body['slots'] ?? null;
        if (!$hasBody || array_key_exists('slots', $body)) {
            if (!is_array($slots) || ($slots !== [] && array_is_list($slots))) {
                throw new ApiError(400, 'invalid_slots', 'slots must be an object { slot: entryId|null }.');
            }
        }
        $target = $hasBody
            ? (is_array($body['body']) ? ['character' => (string) $body['body']['character'], 'look' => (string) $body['body']['look']] : null)
            : $this->body($userId);
        $instances = $this->instances($target ?? $this->unlocks->defaultBody($userId));
        $types = array_column($instances, 'slot', 'id');

        $changes = [];
        foreach (is_array($slots) ? $slots : [] as $id => $entryId) {
            $id = (string) $id;
            if (!isset($types[$id])) throw new ApiError(422, 'invalid_slot', "Unknown slot $id.", ['slot' => $id, 'slots' => array_keys($types)]);
            if ($entryId === null) { $changes[$id] = null; continue; }
            if (!is_string($entryId)) throw new ApiError(422, 'not_owned', 'Unknown inventory entry.', ['slot' => $id]);
            $entry = $this->db->one('SELECT * FROM inventory WHERE id = ? AND user_id = ? AND qty > 0', [$entryId, $userId]);
            if (!$entry) throw new ApiError(422, 'not_owned', 'This item is not in your inventory.', ['slot' => $id]);
            $item = $this->catalog->item((string) $entry['item_id']);
            if (!$item || ($item['slot'] ?? null) !== $types[$id]) {
                throw new ApiError(422, 'wrong_slot', 'This item cannot be worn there.', ['slot' => $id, 'slot_type' => $types[$id], 'item_slot' => $item['slot'] ?? null]);
            }
            $changes[$id] = $entryId;
        }

        $this->db->transaction(function () use ($userId, $changes, $hasBody, $body, $instances): void {
            if ($hasBody) $this->saveBody($userId, $body['body']);
            // the worn items on the (new) body, then the changes; one entry is worn once
            $worn = $this->remap($this->stored($userId), $instances);
            foreach ($changes as $id => $entryId) {
                if ($entryId !== null) {
                    foreach ($worn as $other => $e) if ($e === $entryId) $worn[$other] = null;
                }
                $worn[$id] = $entryId;
            }
            $this->db->run('DELETE FROM avatar_slots WHERE user_id = ?', [$userId]);
            $now = $this->clock->utc();
            foreach ($worn as $id => $entryId) {
                if ($entryId !== null) $this->db->run('INSERT INTO avatar_slots (user_id, slot, entry_id, updated_at) VALUES (?, ?, ?, ?)', [$userId, $id, $entryId, $now]);
            }
        });
    }

    /**
     * The stored rows (instance id, entry id, the entry's item slot type), oldest first; rows of entries that are gone
     * are skipped.
     * @return list<array{id: string, entry: string, type: string|null}>
     */
    private function stored(string $userId): array
    {
        $rows = $this->db->all(
            'SELECT a.slot, a.entry_id, i.item_id FROM avatar_slots a JOIN inventory i ON i.id = a.entry_id AND i.user_id = a.user_id AND i.qty > 0 WHERE a.user_id = ? ORDER BY a.updated_at, a.slot',
            [$userId]
        );
        $out = [];
        foreach ($rows as $r) {
            $item = $this->catalog->item((string) $r['item_id']);
            $type = is_string($item['slot'] ?? null) ? $item['slot'] : null;
            $out[] = ['id' => (string) $r['slot'], 'entry' => (string) $r['entry_id'], 'type' => $type];
        }
        return $out;
    }

    /**
     * Worn items onto a body's instances: the same id (with a fitting type) stays, else the first free instance of the
     * item's slot type, else off. Every instance id is in the result (null = empty).
     * @param list<array{id: string, entry: string, type: string|null}> $worn
     * @param list<array{id: string, slot: string}> $instances
     * @return array<string, string|null>
     */
    private function remap(array $worn, array $instances): array
    {
        $out = array_fill_keys(array_column($instances, 'id'), null);
        $types = array_column($instances, 'slot', 'id');
        $placed = [];
        $pending = [];
        foreach ($worn as $w) {
            if (isset($placed[$w['entry']])) continue;
            $id = $w['id'];
            if (array_key_exists($id, $out) && $out[$id] === null && ($w['type'] === null || $w['type'] === $types[$id])) {
                $out[$id] = $w['entry'];
                $placed[$w['entry']] = true;
            } else {
                $pending[] = $w;
            }
        }
        foreach ($pending as $w) {
            if (isset($placed[$w['entry']]) || $w['type'] === null) continue;
            foreach ($instances as $i) {
                if ($i['slot'] === $w['type'] && $out[$i['id']] === null) {
                    $out[$i['id']] = $w['entry'];
                    $placed[$w['entry']] = true;
                    break;
                }
            }
        }
        return $out;
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
