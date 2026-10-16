<?php
declare(strict_types=1);

namespace Scutoons\Domain\Assets;

use Scutoons\Domain\Catalog\CatalogStore;
use Scutoons\Domain\Content\EntityKey;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Database\Database;
use Throwable;

/**
 * The usage index (agents/ASSETS.md "Usages"): every media path referenced by
 *   - the content store: per entity its PUBLISHED version (state `published`) and, when newer, its working copy
 *     (state `draft`) – history versions are not indexed (restoring an old version may bring back a deleted path;
 *     the delete check says so);
 *   - the catalog rows (CatalogStore: items, recipes, albums, products, editions, featured, toons – agents/CATALOG.md):
 *     source `catalog`, ref `<kind label>:<id>` (`item:camera`, `product:dago_hoodie` …), field = the JSON pointer
 *     in the entry's data, state `config` (until 2026-10-04 the catalog files, ref = the file name);
 *   - app code (CODE_REFERENCES – paths the viewer builds itself, e.g. the camera's MobileNet model): source `code`,
 *     state `config`, so they never show as unused and block a delete like the catalog does.
 * A reference is any string with a media extension, and a map layer's `tiles: { path }` (a CDN-root folder, MAP.md
 * §8.9a): every known tile of that set (AssetPaths::tileSet) is used – the viewer builds the tile URLs itself. Content paths are relative to the document's toon folder (the
 * nearest enclosing `toon` field wins – map locations); a path that only exists from the CDN root (registry, catalog
 * style) resolves there. `asset-requests` (wanted images) are wishes, not usages. Rebuilt by `assets:scan`, refreshed
 * per entity on every creator save / publish.
 */
final class AssetUsages
{
    public const SKIP_KINDS = ['asset-requests'];
    /** the catalog kinds whose rows may reference media (permissions / code types / meta never do) */
    public const CATALOG_KINDS = ['items', 'recipes', 'albums', 'products', 'editions', 'featured', 'toons', 'home-scenes', 'airlift-carriers'];
    /**
     * CDN paths (a trailing `/` = everything below) that app code references directly → the code file (repo-relative).
     * Keep in step with the code: AssetsTest checks every entry still appears in its file.
     */
    public const CODE_REFERENCES = [
        'scutoons/models/' => 'viewer/src/lib/components/camera/object-detector.ts',
        'scutoons/fonts/' => 'shared/tokens/vite.js',
        'catpope/vectors/vector-demon-saw-1.svg' => 'viewer/src/lib/components/toon/map/map-overlay.svelte',
        'scutoons/3d/mannequin.glb' => 'shared/content/src/characters.ts',
        'scutoons/3d/mannequin-more-legs.glb' => 'shared/content/src/characters.ts',
        'scutoons/3d/mannequin-wings.glb' => 'shared/content/src/characters.ts',
        'scutoons/3d/table-desert.webp' => 'viewer/src/lib/components/items/record-stage.svelte',
        'scutoons/3d/table-jungle.webp' => 'viewer/src/lib/components/items/record-stage.svelte',
    ];

    /** @var array<string, true>|null */
    private ?array $known = null;
    /** @var array<string, list<string>>|null the known map tiles per tile set */
    private ?array $tiles = null;

    public function __construct(
        private readonly Database $db,
        private readonly Clock $clock,
        private readonly Settings $settings,
        private readonly AssetRegistry $registry,
        private readonly CatalogStore $catalog,
    ) {
    }

    /** @return list<string> URL prefixes of the CDN (absolute references are accepted when they start with one) */
    private function publicBases(): array
    {
        return array_values(array_filter(array_unique([$this->settings->get('CDN_PUBLIC_URL'), $this->settings->get('CDN_URL'), ...AssetPaths::PUBLIC_BASES])));
    }

    /** @return array<string, true> the files the registry knows (current versions) */
    private function known(): array
    {
        if ($this->known === null) {
            $this->known = array_fill_keys($this->registry->currentPaths(), true);
            $this->tiles = null;
        }
        return $this->known;
    }

    /** @return list<string> the known tiles of a tile set (`catpope/map/terrain/painted/v2`) */
    private function tilesOf(string $set): array
    {
        $known = $this->known();
        if ($this->tiles === null) {
            $this->tiles = [];
            foreach (array_keys($known) as $path) {
                $of = AssetPaths::tileSet((string) $path);
                if ($of !== null) $this->tiles[$of][] = (string) $path;
            }
        }
        return $this->tiles[$set] ?? [];
    }

    /**
     * Rebuilds the whole index
     * @return array{content: int, catalog: int, code: int, entities: int, missing: int}
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
        $code = $this->codeUsages();
        $this->db->transaction(function () use ($rows, $catalog, $code) {
            $this->db->run('DELETE FROM asset_usages');
            foreach ([...$rows, ...$catalog, ...$code] as $row) $this->insert($row);
        });
        $known = $this->known();
        $missing = count(array_filter([...$rows, ...$catalog, ...$code], fn(array $r) => !isset($known[$r['path']])));
        return ['content' => count($rows), 'catalog' => count($catalog), 'code' => count($code), 'entities' => $entities, 'missing' => $missing];
    }

    /** @return list<array<string, mixed>> one row per known file below a CODE_REFERENCES prefix / per exact path */
    private function codeUsages(): array
    {
        $rows = [];
        foreach (self::CODE_REFERENCES as $reference => $file) {
            $paths = str_ends_with($reference, '/')
                ? array_values(array_filter(array_keys($this->known()), fn(string $p) => str_starts_with($p, $reference)))
                : [$reference];
            foreach ($paths as $path) {
                $rows[] = ['path' => $path, 'source' => 'code', 'ref' => $file, 'field' => '', 'state' => 'config', 'version_id' => null, 'toon' => null, 'raw' => $reference];
            }
        }
        return $rows;
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

    /** Refreshes the catalog usages (after a catalog write, e.g. an admin edit or a relink) */
    public function reindexCatalog(): void
    {
        $this->known = null;
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
        // a catalog document (item:<id> …, CatalogDocs): its published data is the table's row (source `catalog`); only
        // a newer draft is indexed here – with the catalog's paths (CDN-root), so a picture only a draft uses stays used
        $catalog = isset(EntityKey::CATALOG_KINDS[(string) $latest['kind']]);
        foreach ($versions as $state => $version) {
            if ($catalog && $state === 'published') continue;
            $doc = json_decode((string) $version['json'], true);
            if (!is_array($doc)) continue;
            foreach ($catalog ? $this->references($doc, '', false) : $this->references($doc, (string) ($version['toon'] ?? '')) as $ref) {
                $out[] = ['source' => 'content', 'ref' => $key, 'state' => $state, 'version_id' => (int) $version['id']] + $ref;
            }
        }
        return $out;
    }

    /** @return list<array<string, mixed>> */
    private function catalogUsages(): array
    {
        $out = [];
        foreach (self::CATALOG_KINDS as $kind) {
            foreach ($this->catalog->rows($kind) as $row) {
                $doc = json_decode($row['data'], true);
                if (!is_array($doc)) continue;
                foreach ($this->references($doc, '', false) as $ref) {
                    $out[] = ['source' => 'catalog', 'ref' => self::catalogRef($kind, $row['id']), 'state' => 'config', 'version_id' => null] + $ref;
                }
            }
        }
        return $out;
    }

    /** The ref of a catalog entry's usages: `<kind label>:<id>` (item:camera) */
    public static function catalogRef(string $kind, string $id): string
    {
        return CatalogStore::KINDS[$kind]['label'] . ':' . $id;
    }

    /**
     * The catalog entry of a usage ref, null when it is none (e.g. an old file-name ref)
     * @return array{0: string, 1: string}|null [kind, id]
     */
    public static function catalogEntry(string $ref): ?array
    {
        [$label, $id] = array_pad(explode(':', $ref, 2), 2, '');
        foreach (self::CATALOG_KINDS as $kind) {
            if (CatalogStore::KINDS[$kind]['label'] === $label && $id !== '') return [$kind, $id];
        }
        return null;
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
                if (isset($node['tiles']) && is_array($node['tiles']) && is_string($node['tiles']['path'] ?? null)) {
                    // a map layer's tile pyramid (MapTiles.path, CDN-root): every tile of the set is in use
                    $set = trim((string) $node['tiles']['path'], '/');
                    foreach ($this->tilesOf($set) as $tile) $out[] = ['path' => $tile, 'field' => $pointer . '/tiles/path', 'toon' => null, 'raw' => $set];
                }
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

    /**
     * What a cleanup may delete, per asset (the admin's Unused tab): files nothing references (content incl. drafts,
     * catalog, app code) –
     *   `current`  files of the current version (of a used asset never its source – the renditions come from it);
     *   `old`      files of archived versions (left behind by a rename) that no current version has;
     *   `archive`  the old bytes a replace kept under .versions/ (size unknown here).
     * `all_unused`: no current file is referenced – the cleanup deletes the whole asset, its history included.
     * Rebuild the index first (AssetService::unused does): a stale index would offer files that are used by now.
     * @return list<array{id: string, slug: string, kind: string, all_unused: bool, files: list<array{path: string, what: string, size: int|null}>, size: int}>
     */
    public function unusedFiles(): array
    {
        $used = [];
        foreach ($this->db->all('SELECT DISTINCT path FROM asset_usages') as $r) $used[(string) $r['path']] = true;
        $current = [];
        $assets = [];
        foreach ($this->db->all("SELECT a.id, a.slug, a.kind, v.path AS source, r.path, r.size FROM assets a JOIN asset_versions v ON v.asset_id = a.id AND v.status = 'current'
                JOIN asset_renditions r ON r.version_id = v.id WHERE a.status = 'active' ORDER BY a.slug, r.path") as $r) {
            $id = (string) $r['id'];
            $path = (string) $r['path'];
            $current[$path] = true;
            $assets[$id] ??= ['id' => $id, 'slug' => (string) $r['slug'], 'kind' => (string) $r['kind'], 'source' => (string) $r['source'], 'used' => false, 'current' => [], 'old' => []];
            if (isset($used[$path])) $assets[$id]['used'] = true;
            else $assets[$id]['current'][$path] = ['path' => $path, 'what' => 'current', 'size' => (int) $r['size']];
        }
        foreach ($this->db->all("SELECT a.id, r.path, r.size FROM assets a JOIN asset_versions v ON v.asset_id = a.id AND v.status = 'archived'
                JOIN asset_renditions r ON r.version_id = v.id WHERE a.status = 'active' ORDER BY r.path") as $r) {
            $path = (string) $r['path'];
            if (isset($assets[(string) $r['id']]) && !isset($current[$path]) && !isset($used[$path])) {
                $assets[(string) $r['id']]['old'][$path] = ['path' => $path, 'what' => 'old', 'size' => (int) $r['size']];
            }
        }
        foreach ($this->db->all("SELECT a.id, v.path, v.size, v.archive_path FROM assets a JOIN asset_versions v ON v.asset_id = a.id AND v.status = 'archived'
                WHERE a.status = 'active' ORDER BY v.number") as $v) {
            $id = (string) $v['id'];
            if (!isset($assets[$id])) continue;
            $path = (string) $v['path'];
            if (!isset($current[$path]) && !isset($used[$path])) $assets[$id]['old'][$path] ??= ['path' => $path, 'what' => 'old', 'size' => (int) $v['size']];
            if ($v['archive_path']) $assets[$id]['old'][(string) $v['archive_path']] = ['path' => (string) $v['archive_path'], 'what' => 'archive', 'size' => null];
        }
        $out = [];
        foreach ($assets as $a) {
            $files = $a['current'];
            if ($a['used']) unset($files[$a['source']]);
            $files = [...array_values($files), ...array_values($a['old'])];
            if (!$files) continue;
            $out[] = ['id' => $a['id'], 'slug' => $a['slug'], 'kind' => $a['kind'], 'all_unused' => !$a['used'], 'files' => $files,
                'size' => array_sum(array_map(fn(array $f) => (int) $f['size'], $files))];
        }
        return $out;
    }
}
