<?php
declare(strict_types=1);

namespace Scutoons\Domain\Library;

use Scutoons\Infrastructure\Database\Database;

/**
 * Which version of every document of some scopes (toon ids, the platform pseudo-toon `scutoons`) a library answer is
 * built from: the published one, or – draft mode, creators – the newest. One indexed query; the documents themselves
 * are read lazily by version id (immutable rows, decoded once per request). The fingerprint (hash of all ids) keys the
 * server cache: a publish anywhere in these scopes gives a new fingerprint.
 */
final class Snapshot
{
    /** @var array<int, mixed> version id → decoded document */
    private array $docs = [];

    /**
     * @param array<string, int> $ids `scope/locale/path` → version id
     * @param array<string, int> $first `scope/locale/path` → id of its oldest version (creation order)
     */
    private function __construct(
        private readonly Database $db,
        public readonly array $ids,
        public readonly bool $draft,
        public readonly array $first = [],
    ) {
    }

    /** @param list<string> $scopes */
    public static function load(Database $db, array $scopes, bool $draft): self
    {
        $in = implode(', ', array_fill(0, count($scopes), '?'));
        $rows = $draft
            ? $db->all("SELECT toon, locale, path, MAX(id) AS id FROM content_versions WHERE toon IN ($in) GROUP BY toon, locale, path", $scopes)
            : $db->all("SELECT toon, locale, path, id FROM content_versions WHERE toon IN ($in) AND status = 'published' ORDER BY id", $scopes);
        return new self($db, self::index($rows), $draft);
    }

    /** Every toon's index entry `<toon>/_/index.json` (the toons list), with the creation order of each */
    public static function toonIndex(Database $db, bool $draft): self
    {
        $where = "locale = '_' AND path = 'index.json'";
        $rows = $draft
            ? $db->all("SELECT toon, locale, path, MAX(id) AS id FROM content_versions WHERE $where GROUP BY toon, locale, path")
            : $db->all("SELECT toon, locale, path, id FROM content_versions WHERE $where AND status = 'published' ORDER BY id");
        $first = self::index($db->all("SELECT toon, locale, path, MIN(id) AS id FROM content_versions WHERE $where GROUP BY toon, locale, path"));
        return new self($db, self::index($rows), $draft, $first);
    }

    /**
     * @param list<array<string, mixed>> $rows
     * @return array<string, int>
     */
    private static function index(array $rows): array
    {
        $ids = [];
        foreach ($rows as $row) $ids[$row['toon'] . '/' . $row['locale'] . '/' . $row['path']] = (int) $row['id'];
        ksort($ids);
        return $ids;
    }

    public function has(string $scope, string $locale, string $path): bool
    {
        return isset($this->ids["$scope/$locale/$path"]);
    }

    /** The decoded document (objects as stdClass, so `{}` stays an object), null when there is none */
    public function doc(string $scope, string $locale, string $path): mixed
    {
        $id = $this->ids["$scope/$locale/$path"] ?? null;
        if ($id === null) return null;
        if (!array_key_exists($id, $this->docs)) {
            $json = $this->db->value('SELECT json FROM content_versions WHERE id = ?', [$id]);
            $this->docs[$id] = is_string($json) ? json_decode($json) : null;
        }
        return $this->docs[$id];
    }

    /** @return list<string> the locales a scope has documents in (without `_`) */
    public function locales(string $scope): array
    {
        $locales = [];
        foreach (array_keys($this->ids) as $key) {
            [$s, $locale] = explode('/', $key, 3);
            if ($s === $scope && $locale !== '_') $locales[$locale] = true;
        }
        return array_keys($locales);
    }

    /** @return list<string> the paths of a scope in one locale */
    public function paths(string $scope, string $locale): array
    {
        $prefix = "$scope/$locale/";
        $paths = [];
        foreach (array_keys($this->ids) as $key) {
            if (str_starts_with($key, $prefix)) $paths[] = substr($key, strlen($prefix));
        }
        return $paths;
    }

    /** @return list<string> the scopes (toon ids) of the snapshot, in creation order when known */
    public function scopes(): array
    {
        $scopes = [];
        foreach ($this->ids as $key => $id) {
            $scope = explode('/', $key, 2)[0];
            $scopes[$scope] = min($scopes[$scope] ?? PHP_INT_MAX, $this->first[$key] ?? $id);
        }
        asort($scopes);
        return array_map('strval', array_keys($scopes));
    }

    public function fingerprint(): string
    {
        return sha1(($this->draft ? 'draft:' : 'published:') . json_encode($this->ids));
    }
}
