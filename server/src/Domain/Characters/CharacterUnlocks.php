<?php
declare(strict_types=1);

namespace Scutoons\Domain\Characters;

use PDOException;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Content\EntityStore;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Database\Database;
use Throwable;

/**
 * Which characters (and which of their A-pose looks) a member may wear as the reader's avatar body
 * (agents/ITEMS.md "Reader avatar bodies & character unlocks").
 *
 *   default   the setting AVATAR_DEFAULT_UNLOCKS – JSON { "<character>": ["<look>", …] } – for every member, no rows
 *             (owner 2026-09-30: catpope with swim + naked)
 *   rows      member_characters (the character with its default A-pose look) + member_character_looks (further
 *             looks), each with `source` (default | code | item | story | admin) and `ref`
 *
 * The character registry (published content store file `scutoons/_/characters.json`) says which A-pose looks exist and
 * have an image; a body must be unlocked AND have an A-pose image.
 */
final class CharacterUnlocks
{
    public const SOURCES = ['default', 'code', 'item', 'story', 'admin'];
    /** The character registry's entity key (published version) */
    public const REGISTRY_KEY = 'characters';
    private const ID = '/^[a-z0-9][a-z0-9-]{0,63}$/';

    /** @var array<string, array<string, mixed>>|null character id → registry character (per request) */
    private ?array $registry = null;

    public function __construct(
        private readonly Database $db,
        private readonly Clock $clock,
        private readonly Settings $settings,
        private readonly EntityStore $content,
    ) {
    }

    /**
     * The configured default unlocks: character → A-pose look ids (invalid JSON = none).
     * @return array<string, list<string>>
     */
    public function defaults(): array
    {
        $decoded = json_decode($this->settings->get('AVATAR_DEFAULT_UNLOCKS'), true);
        if (!is_array($decoded)) return [];
        $out = [];
        foreach ($decoded as $character => $looks) {
            if (!is_string($character) || !preg_match(self::ID, $character) || !is_array($looks)) continue;
            $out[$character] = array_values(array_filter($looks, fn($l) => is_string($l) && preg_match(self::ID, $l) === 1));
        }
        return $out;
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
            $row = $this->content->published(self::REGISTRY_KEY);
        } catch (Throwable) {
            $row = null; // entity_versions not migrated yet
        }
        $data = $row ? json_decode((string) $row['json'], true) : null;
        foreach ((is_array($data) ? ($data['characters'] ?? []) : []) as $c) {
            if (is_array($c) && is_string($c['id'] ?? null)) $this->registry[$c['id']] = $c;
        }
        return $this->registry;
    }

    /** @return list<array<string, mixed>> the character's A-pose looks */
    private function aposeLooks(string $character): array
    {
        $looks = $this->registry()[$character]['apose']['looks'] ?? [];
        return is_array($looks) ? array_values(array_filter($looks, 'is_array')) : [];
    }

    /** The default A-pose look id of a character (marked default, else the first; null = no A-pose) */
    public function defaultLook(string $character): ?string
    {
        $looks = $this->aposeLooks($character);
        foreach ($looks as $l) if (($l['default'] ?? false) === true && is_string($l['id'] ?? null)) return $l['id'];
        return is_string($looks[0]['id'] ?? null) ? $looks[0]['id'] : null;
    }

    /** true when the character has this A-pose look with an image */
    public function hasApose(string $character, string $look): bool
    {
        foreach ($this->aposeLooks($character) as $l) {
            if (($l['id'] ?? null) === $look) return is_string($l['path'] ?? null) && trim($l['path']) !== '';
        }
        return false;
    }

    /**
     * Everything the member has unlocked, defaults first, then rows (unlock order):
     * [{ character, looks: [look ids], source, ref, unlocked_at }] – `source` / `ref` of the character unlock.
     * @return list<array<string, mixed>>
     */
    public function forUser(string $userId): array
    {
        /** @var array<string, array<string, mixed>> $out */
        $out = [];
        foreach ($this->defaults() as $character => $looks) {
            $out[$character] = ['character' => $character, 'looks' => $looks, 'source' => 'default', 'ref' => null, 'unlocked_at' => null];
        }
        try {
            $characters = $this->db->all('SELECT character_id, source, ref, unlocked_at FROM member_characters WHERE user_id = ? ORDER BY unlocked_at, character_id', [$userId]);
            $looks = $this->db->all('SELECT character_id, look_id FROM member_character_looks WHERE user_id = ? ORDER BY unlocked_at, look_id', [$userId]);
        } catch (PDOException) {
            return array_values($out); // tables not migrated yet: defaults only
        }
        foreach ($characters as $r) {
            $id = (string) $r['character_id'];
            if (!isset($out[$id])) {
                $default = $this->defaultLook($id);
                $out[$id] = ['character' => $id, 'looks' => $default !== null ? [$default] : [], 'source' => (string) $r['source'], 'ref' => $r['ref'], 'unlocked_at' => $r['unlocked_at']];
            }
        }
        foreach ($looks as $r) {
            $id = (string) $r['character_id'];
            // a look without the character unlock also unlocks the character (with just that look)
            $out[$id] ??= ['character' => $id, 'looks' => [], 'source' => 'admin', 'ref' => null, 'unlocked_at' => null];
            if (!in_array($r['look_id'], $out[$id]['looks'], true)) $out[$id]['looks'][] = (string) $r['look_id'];
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

    public function isUnlocked(string $userId, string $character, string $look): bool
    {
        foreach ($this->forUser($userId) as $u) if ($u['character'] === $character) return in_array($look, $u['looks'], true);
        return false;
    }

    /**
     * Admin: unlock a character (and optionally further A-pose looks) for a member. 400 unknown_character (not in the
     * registry) | invalid_look | invalid_source. Returns whether anything changed.
     * @param list<string> $looks
     */
    public function grant(string $userId, string $character, array $looks, string $source, ?string $ref): bool
    {
        $this->checkCharacter($character);
        if (!in_array($source, self::SOURCES, true)) throw new ApiError(400, 'invalid_source', 'Unknown unlock source.', ['sources' => self::SOURCES]);
        $known = array_filter(array_map(fn(array $l) => $l['id'] ?? null, $this->aposeLooks($character)), 'is_string');
        foreach ($looks as $look) {
            if (!is_string($look) || !in_array($look, $known, true)) throw new ApiError(400, 'invalid_look', "Unknown A-pose look of $character.", ['looks' => array_values($known)]);
        }
        $now = $this->clock->utc();
        return (bool) $this->db->transaction(function () use ($userId, $character, $looks, $source, $ref, $now) {
            $changed = false;
            if (!$this->db->one('SELECT 1 FROM member_characters WHERE user_id = ? AND character_id = ?', [$userId, $character])) {
                $this->db->run('INSERT INTO member_characters (user_id, character_id, source, ref, unlocked_at) VALUES (?, ?, ?, ?, ?)', [$userId, $character, $source, $ref, $now]);
                $changed = true;
            }
            foreach (array_unique($looks) as $look) {
                if ($look === $this->defaultLook($character)) continue; // the character unlock includes its default look
                if ($this->db->one('SELECT 1 FROM member_character_looks WHERE user_id = ? AND character_id = ? AND look_id = ?', [$userId, $character, $look])) continue;
                $this->db->run('INSERT INTO member_character_looks (user_id, character_id, look_id, source, ref, unlocked_at) VALUES (?, ?, ?, ?, ?, ?)', [$userId, $character, $look, $source, $ref, $now]);
                $changed = true;
            }
            return $changed;
        });
    }

    /**
     * Admin: revoke a character unlock (with its looks), or only one look. Default unlocks can't be revoked per member
     * (change the setting). A worn body that is no longer unlocked is reset.
     */
    public function revoke(string $userId, string $character, ?string $look = null): bool
    {
        if (!preg_match(self::ID, $character)) throw new ApiError(400, 'unknown_character', 'Unknown character.');
        $changed = $this->db->transaction(function () use ($userId, $character, $look) {
            if ($look !== null) {
                return $this->db->run('DELETE FROM member_character_looks WHERE user_id = ? AND character_id = ? AND look_id = ?', [$userId, $character, $look])->rowCount() > 0;
            }
            $a = $this->db->run('DELETE FROM member_characters WHERE user_id = ? AND character_id = ?', [$userId, $character])->rowCount();
            $b = $this->db->run('DELETE FROM member_character_looks WHERE user_id = ? AND character_id = ?', [$userId, $character])->rowCount();
            return $a + $b > 0;
        });
        $body = $this->db->one('SELECT character_id, look_id FROM avatar_body WHERE user_id = ?', [$userId]);
        if ($body && !$this->isUnlocked($userId, (string) $body['character_id'], (string) $body['look_id'])) {
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
