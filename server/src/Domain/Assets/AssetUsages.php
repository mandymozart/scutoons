<?php
declare(strict_types=1);

namespace Scutoons\Domain\Assets;

use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Database\Database;
use Throwable;

/**
 * The usage index (agents/ASSETS.md "Usages"): every media path referenced by
 *   - the content store: per entity its PUBLISHED version (state `published`) and, when newer, its working copy
 *     (state `draft`) – history versions are not indexed (restoring an old version may bring back a deleted path;
 *     the delete check says so);
 *   - the catalog files (items.json incl. albums, products.json, editions.json, toons.json, featured-products.json):
 *     state `config`.
 * A reference is any string with a media extension. Content paths are relative to the document's toon folder (the
 * nearest enclosing `toon` field wins – map locations); a path that only exists from the CDN root (registry, catalog
 * style) resolves there. `asset-requests` (wanted images) are wishes, not usages. Rebuilt by `assets:scan`, refreshed
 * per entity on every creator save / publish.
 */
final class AssetUsages
{
    public const SKIP_KINDS = ['asset-requests'];
    public const CATALOG_FILES = ['ITEMS_PATH', 'PRODUCTS_PATH', 'EDITIONS_PATH', 'TOONS_PATH', 'FEATURED_PRODUCTS_PATH'];

    /** @var array<string, true>|null */
    private ?array $known = null;

    public function __construct(
        private readonly Database $db,
        private readonly Clock $clock,
        private readonly Settings $settings,
        private readonly AssetRegistry $registry,
    ) {
    }

    /** @return list<string> URL prefixes of the CDN (absolute references are accepted when they start with one) */
    private function publicBases(): array
    {
        return array_values(array_filter(array_unique([$this->settings->get('CDN_PUBLIC_URL'), $this->settings->get('CDN_URL'), 'https://cdn.scutoons.com/'])));
    }

    /** @return array<string, true> the files the registry knows (current versions) */
    private function known(): array
    {
        if ($this->known === null) {
            $this->known = array_fill_keys($this->registry->currentPaths(), true);
        }
        return $this->known;
    }

    /**
     * Rebuilds the whole index
     * @return array{content: int, catalog: int, entities: int, missing: int}
     */
    public function rebuild(): array
    {
        $this->known = null;
        $rows = [];
        $entities = 0;
        foreach ($this->db->all('SELECT DISTINCT entity_key FROM entity_versions ORDER BY entity_key') as $r) {
            $found = $this->entityUsages((string) $r['entity_key']);
            if ($found === null) continue;
            $entities++;
            array_push($rows, ...$found);
        }
        $catalog = $this->catalogUsages();
        $this->db->transaction(function () use ($rows, $catalog) {
            $this->db->run('DELETE FROM asset_usages');
            foreach ([...$rows, ...$catalog] as $row) $this->insert($row);
        });
        $known = $this->known();
        $missing = count(array_filter([...$rows, ...$catalog], fn(array $r) => !isset($known[$r['path']])));
        return ['content' => count($rows), 'catalog' => count($catalog), 'entities' => $entities, 'missing' => $missing];
    }

    /** Refreshes one entity's rows (after a creator save / publish / restore). Silent when the tables are missing. */
    public function reindexEntity(string $key): void
    {
        try {
            $this->known = null;
            $rows = $this->entityUsages($key) ?? [];
            $this->db->transaction(function () use ($key, $rows) {
                $this->db->run("DELETE FROM asset_usages WHERE source = 'content' AND ref = ?", [$key]);
                foreach ($rows as $row) $this->insert($row);
            });
        } catch (Throwable) {
            // the usage index must never break a content save (no migration yet → nothing to index)
        }
    }

    /** Refreshes the catalog rows (after a catalog file changed, e.g. a relink) */
    public function reindexCatalog(): void
    {
        $rows = $this->catalogUsages();
        $this->db->transaction(function () use ($rows) {
            $this->db->run("DELETE FROM asset_usages WHERE source = 'catalog'");
            foreach ($rows as $row) $this->insert($row);
        });
    }

    /** @param array<string, mixed> $row */
    private function insert(array $row): void
    {
        $this->db->run('INSERT INTO asset_usages (path, source, ref, field, state, version_id, toon, raw, scanned_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)', [
            $row['path'], $row['source'], $row['ref'], $row['field'], $row['state'], $row['version_id'], $row['toon'], mb_substr((string) $row['raw'], 0, 500), $this->clock->utc(),
        ]);
    }

    /** @return list<array<string, mixed>>|null null = an entity kind that is never indexed */
    private function entityUsages(string $key): ?array
    {
        $latest = $this->db->one('SELECT id, kind, toon, json FROM entity_versions WHERE entity_key = ? ORDER BY id DESC LIMIT 1', [$key]);
        if (!$latest || in_array($latest['kind'], self::SKIP_KINDS, true)) return null;
        $published = $this->db->one("SELECT id, kind, toon, json FROM entity_versions WHERE entity_key = ? AND status = 'published' ORDER BY id DESC LIMIT 1", [$key]);
        $out = [];
        $versions = [];
        if ($published) $versions['published'] = $published;
        if (!$published || (int) $published['id'] !== (int) $latest['id']) $versions['draft'] = $latest;
        foreach ($versions as $state => $version) {
            $doc = json_decode((string) $version['json'], true);
            if (!is_array($doc)) continue;
            foreach ($this->references($doc, (string) ($version['toon'] ?? '')) as $ref) {
                $out[] = ['source' => 'content', 'ref' => $key, 'state' => $state, 'version_id' => (int) $version['id']] + $ref;
            }
        }
        return $out;
    }

    /** @return list<array<string, mixed>> */
    private function catalogUsages(): array
    {
        $out = [];
        foreach (self::CATALOG_FILES as $setting) {
            $file = $this->settings->path($setting);
            if ($file === '' || !is_file($file)) continue;
            $doc = json_decode((string) file_get_contents($file), true);
            if (!is_array($doc)) continue;
            foreach ($this->references($doc, '', false) as $ref) {
                $out[] = ['source' => 'catalog', 'ref' => basename($file), 'state' => 'config', 'version_id' => null] + $ref;
            }
        }
        return $out;
    }

    /**
     * Every media string of a document: { path, field (JSON pointer), toon (the base folder it resolved against, null =
     * the CDN root), raw }
     * @param array<mixed> $doc
     * @return list<array{path: string, field: string, toon: ?string, raw: string}>
     */
    public function references(array $doc, string $base, bool $toonContext = true): array
    {
        $out = [];
        $walk = function (mixed $node, string $pointer, string $base) use (&$walk, &$out, $toonContext): void {
            if (is_array($node)) {
                if ($toonContext && isset($node['toon']) && is_string($node['toon']) && preg_match('/^[a-z0-9_-]{1,64}$/', $node['toon'])) $base = $node['toon'];
                foreach ($node as $k => $v) $walk($v, $pointer . '/' . strtr((string) $k, ['~' => '~0', '/' => '~1']), $base);
                return;
            }
            if (!is_string($node) || !preg_match(AssetPaths::MEDIA_EXTENSION, trim((string) preg_replace('/[?#].*$/', '', $node)))) return;
            [$path, $resolvedBase] = $this->resolve($node, $base);
            if ($path !== null) $out[] = ['path' => $path, 'field' => $pointer === '' ? '/' : $pointer, 'toon' => $resolvedBase !== '' ? $resolvedBase : null, 'raw' => $node];
        };
        $walk($doc, '', $base);
        return $out;
    }

    /** @return array{0: ?string, 1: string} the key + the base it was resolved against */
    public function resolve(string $raw, string $base): array
    {
        $bases = $this->publicBases();
        $primary = AssetPaths::resolve($raw, $base, $bases);
        if ($base === '' || $primary === null) return [$primary, ''];
        $known = $this->known();
        if (isset($known[$primary])) return [$primary, $base];
        $root = AssetPaths::resolve($raw, '', $bases);
        if ($root !== null && isset($known[$root]) && !str_starts_with(trim($raw), '../')) return [$root, ''];
        return [$primary, $base];
    }

    // ── Queries ───────────────────────────────────────────────────────────────────────────────────────────

    /**
     * References to files the registry does not know (missing on the storage or never scanned)
     * @return list<array{path: string, usages: list<array<string, mixed>>}>
     */
    public function missing(): array
    {
        $known = array_fill_keys($this->registry->currentPaths(), true);
        $out = [];
        foreach ($this->db->all('SELECT * FROM asset_usages ORDER BY path, source, ref, field') as $u) {
            if (isset($known[(string) $u['path']])) continue;
            $out[(string) $u['path']][] = ['source' => $u['source'], 'ref' => $u['ref'], 'field' => $u['field'], 'state' => $u['state'], 'raw' => $u['raw']];
        }
        return array_map(fn(string $path, array $usages) => ['path' => $path, 'usages' => $usages], array_keys($out), array_values($out));
    }

    /**
     * Files of current versions nothing references (orphans), per asset
     * @return list<array{id: string, slug: string, kind: string, unused: list<string>, all_unused: bool, size: int}>
     */
    public function orphans(): array
    {
        $used = [];
        foreach ($this->db->all('SELECT DISTINCT path FROM asset_usages') as $r) $used[(string) $r['path']] = true;
        $out = [];
        foreach ($this->db->all("SELECT a.id, a.slug, a.kind, r.path, r.size FROM assets a JOIN asset_versions v ON v.asset_id = a.id AND v.status = 'current'
                JOIN asset_renditions r ON r.version_id = v.id WHERE a.status = 'active' ORDER BY a.slug, r.path") as $r) {
            $id = (string) $r['id'];
            $out[$id] ??= ['id' => $id, 'slug' => (string) $r['slug'], 'kind' => (string) $r['kind'], 'unused' => [], 'files' => 0, 'size' => 0];
            $out[$id]['files']++;
            if (!isset($used[(string) $r['path']])) {
                $out[$id]['unused'][] = (string) $r['path'];
                $out[$id]['size'] += (int) $r['size'];
            }
        }
        $result = [];
        foreach ($out as $row) {
            if (!$row['unused']) continue;
            $result[] = ['id' => $row['id'], 'slug' => $row['slug'], 'kind' => $row['kind'], 'unused' => $row['unused'], 'all_unused' => count($row['unused']) === $row['files'], 'size' => $row['size']];
        }
        return $result;
    }
}
