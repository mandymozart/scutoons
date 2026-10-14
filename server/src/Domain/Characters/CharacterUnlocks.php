<?php
declare(strict_types=1);

namespace Scutoons\Domain\Characters;

use PDOException;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Content\CollectionDocs;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Database\Database;
use Throwable;

/**
 * Which characters (and which of their looks) a member may wear as the reader's avatar body (agents/ITEMS.md "Reader
 * avatar bodies & character unlocks"). A body is a look with an A-pose (`personas[].looks[].apose`, owner 2026-10-12:
 * "a poses come per look"); a look is named `<persona>/<look>` in unlock lists (`bodyLookKey` in @scutoons/content).
 *
 *   default   the setting AVATAR_DEFAULT_UNLOCKS – JSON ["<character>", …] – the characters every member has, with
 *             their default body; no rows (owner 2026-10-05: catpope)
 *   rows      member_characters (the character with its default body) + member_character_looks (further looks:
 *             persona_id + look_id), each with `source` (default | code | item | story | admin) and `ref`
 *
 * A character's default body: its regular persona's default look when that has an A-pose image, else its first look
 * with one (registry order). The published character registry says which looks have an A-pose image; a body must be
 * unlocked AND have one.
 */
final class CharacterUnlocks
{
    public const SOURCES = ['default', 'code', 'item', 'story', 'admin'];
    /** The character registry's entity key (published version) */
    public const REGISTRY_KEY = 'characters';
    public const REGULAR_PERSONA = 'regular';
    private const ID = '/^[a-z0-9][a-z0-9-]{0,63}$/';

    /** @var array<string, array<string, mixed>>|null character id → registry character (per request) */
    private ?array $registry = null;

    public function __construct(
        private readonly Database $db,
        private readonly Clock $clock,
        private readonly Settings $settings,
    ) {
    }

    /** `<persona>/<look>` – a look in unlock lists */
    public static function key(string $persona, string $look): string
    {
        return "$persona/$look";
    }

    /**
     * The configured default unlocks: the character ids every member has (invalid JSON = none).
     * @return list<string>
     */
    public function defaults(): array
    {
        $decoded = json_decode($this->settings->get('AVATAR_DEFAULT_UNLOCKS'), true);
        if (!is_array($decoded) || !array_is_list($decoded)) return [];
        return array_values(array_unique(array_filter($decoded, fn($c) => is_string($c) && preg_match(self::ID, $c) === 1)));
    }

    /**
     * Registry characters by id (published store file; none when the store or the file is missing).
     * @return array<string, array<string, mixed>>
     */
    public function registry(): array
    {
        if ($this->registry !== null) return $this->registry;
        $this->registry = [];
        try {
            // the published registry, composed from `characters` + character:<id> (agents/MAP-DOCUMENTS.md)
            $registry = CollectionDocs::published(CollectionDocs::CHARACTERS, $this->db);
        } catch (Throwable) {
            $registry = null; // entity_versions not migrated yet
        }
        $data = $registry !== null ? json_decode((string) json_encode($registry), true) : null;
        foreach ((is_array($data) ? ($data['characters'] ?? []) : []) as $c) {
            if (is_array($c) && is_string($c['id'] ?? null)) $this->registry[$c['id']] = $c;
        }
        return $this->registry;
    }

    /**
     * The character's looks with an A-pose image (registry order: persona → look).
     * @return list<array{persona: string, look: string, default: bool, apose: array<string, mixed>}> `default` = the persona's default look
     */
    private function bodies(string $character): array
    {
        $out = [];
        foreach ($this->registry()[$character]['personas'] ?? [] as $p) {
            if (!is_array($p) || !is_string($p['id'] ?? null)) continue;
            foreach (is_array($p['looks'] ?? null) ? $p['looks'] : [] as $l) {
                $apose = is_array($l) ? ($l['apose'] ?? null) : null;
                if (!is_string($l['id'] ?? null) || !is_array($apose) || !is_string($apose['path'] ?? null) || trim($apose['path']) === '') continue;
                $out[] = ['persona' => $p['id'], 'look' => $l['id'], 'default' => ($l['default'] ?? false) === true, 'apose' => $apose];
            }
        }
        return $out;
    }

    /**
     * The default body of a character: the regular persona's default look with an A-pose image, else the first look with
     * one; null = no A-pose at all.
     * @return array{persona: string, look: string}|null
     */
    public function defaultLook(string $character): ?array
    {
        $bodies = $this->bodies($character);
        foreach ($bodies as $b) if ($b['persona'] === self::REGULAR_PERSONA && $b['default']) return ['persona' => $b['persona'], 'look' => $b['look']];
        return $bodies ? ['persona' => $bodies[0]['persona'], 'look' => $bodies[0]['look']] : null;
    }

    /** true when the look has an A-pose image */
    public function hasApose(string $character, string $persona, string $look): bool
    {
        foreach ($this->bodies($character) as $b) if ($b['persona'] === $persona && $b['look'] === $look) return true;
        return false;
    }

    /**
     * The wear slots (slot instances) of a body: the A-pose's `slots` as [{ id, slot }] (registry order; a duo has
     * `torso-left` + `torso-right`, a single figure `torso`). Empty = the look has none (or is unknown) – the caller
     * falls back to one instance per catalog slot type (agents/ITEMS.md "Slots per avatar body").
     * @return list<array{id: string, slot: string}>
     */
    public function slotInstances(string $character, string $persona, string $look): array
    {
        foreach ($this->bodies($character) as $b) {
            if ($b['persona'] !== $persona || $b['look'] !== $look) continue;
            $out = [];
            foreach (is_array($b['apose']['slots'] ?? null) ? $b['apose']['slots'] : [] as $s) {
                if (is_array($s) && is_string($s['id'] ?? null) && is_string($s['slot'] ?? null)) $out[] = ['id' => $s['id'], 'slot' => $s['slot']];
            }
            return $out;
        }
        return [];
    }

    /**
     * The body the reader sees when none is chosen (the viewer's rule): the first unlocked character's default body,
     * else the first unlocked look with an A-pose image; null = none.
     * @return array{character: string, persona: string, look: string}|null
     */
    public function defaultBody(string $userId): ?array
    {
        $unlocked = $this->forUser($userId);
        foreach ($unlocked as $u) {
            $d = $this->defaultLook($u['character']);
            if ($d !== null) return ['character' => $u['character']] + $d;
        }
        foreach ($unlocked as $u) {
            foreach ($u['looks'] as $key) {
                [$persona, $look] = explode('/', $key, 2);
                if ($this->hasApose($u['character'], $persona, $look)) return ['character' => $u['character'], 'persona' => $persona, 'look' => $look];
            }
        }
        return null;
    }

    /**
     * Everything the member has unlocked, defaults first, then rows (unlock order):
     * [{ character, looks: [`<persona>/<look>`], source, ref, unlocked_at }] – `looks` starts with the character's default
     * body (when it has one); `source` / `ref` of the character unlock.
     * @return list<array<string, mixed>>
     */
    public function forUser(string $userId): array
    {
        /** @var array<string, array<string, mixed>> $out */
        $out = [];
        $add = function (string $character, string $source, mixed $ref, mixed $at) use (&$out): void {
            if (isset($out[$character])) return;
            $d = $this->defaultLook($character);
            $out[$character] = ['character' => $character, 'looks' => $d !== null ? [self::key($d['persona'], $d['look'])] : [], 'source' => $source, 'ref' => $ref, 'unlocked_at' => $at];
        };
        foreach ($this->defaults() as $character) $add($character, 'default', null, null);
        try {
            $characters = $this->db->all('SELECT character_id, source, ref, unlocked_at FROM member_characters WHERE user_id = ? ORDER BY unlocked_at, character_id', [$userId]);
            $looks = $this->db->all('SELECT character_id, persona_id, look_id FROM member_character_looks WHERE user_id = ? ORDER BY unlocked_at, persona_id, look_id', [$userId]);
        } catch (PDOException) {
            return array_values($out); // tables not migrated yet: defaults only
        }
        foreach ($characters as $r) $add((string) $r['character_id'], (string) $r['source'], $r['ref'], $r['unlocked_at']);
        foreach ($looks as $r) {
            $id = (string) $r['character_id'];
            // a look without the character unlock also unlocks the character (with that look)
            $add($id, 'admin', null, null);
            $key = self::key((string) $r['persona_id'], (string) $r['look_id']);
            if (!in_array($key, $out[$id]['looks'], true)) $out[$id]['looks'][] = $key;
        }
        return array_values($out);
    }

    /**
     * GET /me/ avatar.unlocked: [{ character, looks, source }]
     * @return list<array{character: string, looks: list<string>, source: string}> */
    public function toJson(string $userId): array
    {
        return array_map(fn(array $u) => ['character' => $u['character'], 'looks' => $u['looks'], 'source' => $u['source']], $this->forUser($userId));
    }

    public function isUnlocked(string $userId, string $character, string $persona, string $look): bool
    {
        foreach ($this->forUser($userId) as $u) if ($u['character'] === $character) return in_array(self::key($persona, $look), $u['looks'], true);
        return false;
    }

    /**
     * Admin: unlock a character (and optionally further looks, `<persona>/<look>`) for a member. 400 unknown_character
     * (not in the registry) | invalid_look (no such look with an A-pose image) | invalid_source. Returns whether
     * anything changed.
     * @param list<string> $looks
     */
    public function grant(string $userId, string $character, array $looks, string $source, ?string $ref): bool
    {
        $this->checkCharacter($character);
        if (!in_array($source, self::SOURCES, true)) throw new ApiError(400, 'invalid_source', 'Unknown unlock source.', ['sources' => self::SOURCES]);
        $known = array_map(fn(array $b) => self::key($b['persona'], $b['look']), $this->bodies($character));
        foreach ($looks as $look) {
            if (!is_string($look) || !in_array($look, $known, true)) throw new ApiError(400, 'invalid_look', "Unknown look of $character (persona/look with an A-pose).", ['looks' => $known]);
        }
        $default = $this->defaultLook($character);
        $defaultKey = $default !== null ? self::key($default['persona'], $default['look']) : null;
        $now = $this->clock->utc();
        return (bool) $this->db->transaction(function () use ($userId, $character, $looks, $source, $ref, $now, $defaultKey) {
            $changed = false;
            if (!$this->db->one('SELECT 1 FROM member_characters WHERE user_id = ? AND character_id = ?', [$userId, $character])) {
                $this->db->run('INSERT INTO member_characters (user_id, character_id, source, ref, unlocked_at) VALUES (?, ?, ?, ?, ?)', [$userId, $character, $source, $ref, $now]);
                $changed = true;
            }
            foreach (array_unique($looks) as $key) {
                if ($key === $defaultKey) continue; // the character unlock includes its default body
                [$persona, $look] = explode('/', $key, 2);
                if ($this->db->one('SELECT 1 FROM member_character_looks WHERE user_id = ? AND character_id = ? AND persona_id = ? AND look_id = ?', [$userId, $character, $persona, $look])) continue;
                $this->db->run('INSERT INTO member_character_looks (user_id, character_id, persona_id, look_id, source, ref, unlocked_at) VALUES (?, ?, ?, ?, ?, ?, ?)', [$userId, $character, $persona, $look, $source, $ref, $now]);
                $changed = true;
            }
            return $changed;
        });
    }

    /**
     * Admin: revoke a character unlock (with its looks), or only one look. Default unlocks can't be revoked per member
     * (change the setting). A worn body that is no longer unlocked is reset.
     */
    public function revoke(string $userId, string $character, ?string $persona = null, ?string $look = null): bool
    {
        if (!preg_match(self::ID, $character)) throw new ApiError(400, 'unknown_character', 'Unknown character.');
        $changed = $this->db->transaction(function () use ($userId, $character, $persona, $look) {
            if ($persona !== null && $look !== null) {
                return $this->db->run('DELETE FROM member_character_looks WHERE user_id = ? AND character_id = ? AND persona_id = ? AND look_id = ?', [$userId, $character, $persona, $look])->rowCount() > 0;
            }
            $a = $this->db->run('DELETE FROM member_characters WHERE user_id = ? AND character_id = ?', [$userId, $character])->rowCount();
            $b = $this->db->run('DELETE FROM member_character_looks WHERE user_id = ? AND character_id = ?', [$userId, $character])->rowCount();
            return $a + $b > 0;
        });
        $body = $this->db->one('SELECT character_id, persona_id, look_id FROM avatar_body WHERE user_id = ?', [$userId]);
        if ($body && !$this->isUnlocked($userId, (string) $body['character_id'], (string) $body['persona_id'], (string) $body['look_id'])) {
            $this->db->run('DELETE FROM avatar_body WHERE user_id = ?', [$userId]);
        }
        return (bool) $changed;
    }

    private function checkCharacter(string $character): void
    {
        if (!preg_match(self::ID, $character) || !isset($this->registry()[$character])) {
            throw new ApiError(400, 'unknown_character', "Unknown character $character.", ['characters' => array_keys($this->registry())]);
        }
    }
}
