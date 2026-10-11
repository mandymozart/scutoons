<?php
declare(strict_types=1);

namespace Scutoons\Domain\Library;

use Scutoons\Domain\Content\EntityKey;
use Scutoons\Infrastructure\Database\Database;

/**
 * Which version of every entity a library answer is built from: the published one, or – draft mode, creators – the
 * newest. One indexed query per answer; the documents are read lazily by version id (immutable rows, decoded once per
 * request). The fingerprint (hash of all ids) keys the server cache: a publish gives a new fingerprint.
 */
final class Snapshot
{
    /** @var array<int, mixed> version id → decoded document */
    private array $docs = [];

    /**
     * @param array<string, int> $ids entity key → version id
     * @param array<string, int> $first entity key → id of its oldest version (creation order)
     */
    private function __construct(
        private readonly Database $db,
        public readonly array $ids,
        public readonly bool $draft,
        public readonly array $first = [],
    ) {
    }

    /** A toon's entities plus the platform-wide ones (characters, people, events …) */
    public static function toon(Database $db, string $toon, bool $draft): self
    {
        return self::where($db, '(toon = ? OR toon IS NULL)', [$toon], $draft);
    }

    /** The platform-wide entities only */
    public static function platform(Database $db, bool $draft): self
    {
        return self::where($db, 'toon IS NULL', [], $draft);
    }

    /**
     * Every toon's index entry `toon:<toon>` (the toons list), with the creation order of each, plus every toon's settings
     * `toon-settings:<toon>:<l>` (older data kept `announce` + the landing `tile` there)
     */
    public static function toonIndex(Database $db, bool $draft): self
    {
        $snapshot = self::where($db, "kind IN ('toon', 'toon-settings')", [], $draft);
        $first = self::index($db->all("SELECT entity_key, MIN(id) AS id FROM entity_versions WHERE kind = 'toon' GROUP BY entity_key"));
        return new self($db, $snapshot->ids, $draft, $first);
    }

    /**
     * The entities of some kinds (the world map: `map` + every toon's index entry for the visibility of its places)
     * @param list<string> $kinds
     */
    public static function kinds(Database $db, array $kinds, bool $draft): self
    {
        $marks = implode(', ', array_fill(0, count($kinds), '?'));
        return self::where($db, "kind IN ($marks)", array_values($kinds), $draft);
    }

    /**
     * Creator-only kinds (the screenplays, EntityKey::CREATOR_ONLY_KINDS) are never part of a snapshot: no library
     * answer can contain them, and saving a script doesn't change the fingerprint (the cache key) of a reader's answer.
     * @param list<string> $params
     */
    private static function where(Database $db, string $where, array $params, bool $draft): self
    {
        $hidden = implode(', ', array_map(fn(string $kind) => "'$kind'", EntityKey::CREATOR_ONLY_KINDS));
        $where = "($where) AND kind NOT IN ($hidden)";
        $rows = $draft
            ? $db->all("SELECT entity_key, MAX(id) AS id FROM entity_versions WHERE $where GROUP BY entity_key", $params)
            : $db->all("SELECT entity_key, id FROM entity_versions WHERE $where AND status = 'published' ORDER BY id", $params);
        return new self($db, self::index($rows), $draft);
    }

    /**
     * @param list<array<string, mixed>> $rows
     * @return array<string, int>
     */
    private static function index(array $rows): array
    {
        $ids = [];
        foreach ($rows as $row) $ids[(string) $row['entity_key']] = (int) $row['id'];
        ksort($ids);
        return $ids;
    }

    /**
     * The same snapshot without the language-neutral chapter / scene / season documents (only their legacy per-locale
     * ones): the legacy answers, for the conversion's equivalence check (content:neutral-check). Another fingerprint.
     */
    public function withoutNeutralDocs(): self
    {
        $ids = array_filter($this->ids, fn(int $id, string $key) => !(EntityKey::isValid($key) && EntityKey::parse($key)->isNeutralDoc()), ARRAY_FILTER_USE_BOTH);
        return new self($this->db, $ids, $this->draft, $this->first);
    }

    public function has(string $key): bool
    {
        return isset($this->ids[$key]);
    }

    /** The decoded document (objects as stdClass, so `{}` stays an object), null when there is none */
    public function doc(string $key): mixed
    {
        $id = $this->ids[$key] ?? null;
        if ($id === null) return null;
        if (!array_key_exists($id, $this->docs)) {
            $json = $this->db->value('SELECT json FROM entity_versions WHERE id = ?', [$id]);
            $this->docs[$id] = is_string($json) ? json_decode($json) : null;
        }
        return $this->docs[$id];
    }

    /** @return list<string> the keys starting with a prefix */
    public function keys(string $prefix): array
    {
        return array_values(array_filter(array_keys($this->ids), fn(string $k) => str_starts_with($k, $prefix)));
    }

    /**
     * The keys starting with a prefix in creation order (the id of each key's first version – the world map's places and
     * routes, WorldMapDocs); one query per prefix
     * @return list<string>
     */
    public function createdOrder(string $prefix): array
    {
        $keys = $this->keys($prefix);
        if ($keys === []) return [];
        $marks = implode(', ', array_fill(0, count($keys), '?'));
        $first = self::index($this->db->all("SELECT entity_key, MIN(id) AS id FROM entity_versions WHERE entity_key IN ($marks) GROUP BY entity_key", $keys));
        usort($keys, fn(string $a, string $b) => ($first[$a] ?? PHP_INT_MAX) <=> ($first[$b] ?? PHP_INT_MAX) ?: strcmp($a, $b));
        return $keys;
    }

    /** @return list<string> the keys in creation order (the toons list) */
    public function ordered(): array
    {
        $keys = [];
        foreach ($this->ids as $key => $id) $keys[$key] = $this->first[$key] ?? $id;
        asort($keys);
        return array_map('strval', array_keys($keys));
    }

    public function fingerprint(): string
    {
        return sha1(($this->draft ? 'draft:' : 'published:') . json_encode($this->ids));
    }
}
