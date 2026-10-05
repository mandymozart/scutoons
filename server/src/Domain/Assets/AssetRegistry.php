<?php
declare(strict_types=1);

namespace Scutoons\Domain\Assets;

use Scutoons\Domain\Admin\ListQuery;
use Scutoons\Domain\ApiError;
use Scutoons\Infrastructure\AssetStorage\StoredFile;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Ids;
use Scutoons\Support\Time;
use Throwable;

/**
 * The media asset registry (agents/ASSETS.md "Data model"): tables assets, asset_versions, asset_renditions (+ reads
 * of asset_usages). An ASSET is the logical medium (the trailer loop), a VERSION one upload of it (replacing the file =
 * a new version, the old one archived), a RENDITION one file of a version (480p / 720p / 1080p, WebP, 2x …; the
 * version's own file is a rendition too). Paths are CDN-root keys – the same strings content references.
 */
final class AssetRegistry
{
    public function __construct(private readonly Database $db, private readonly Clock $clock)
    {
    }

    /** Without the migration the asset manager answers 503 – everything else of the API keeps working */
    public function available(): bool
    {
        try {
            $this->db->value('SELECT COUNT(*) FROM assets WHERE 1 = 0');
            return true;
        } catch (Throwable) {
            return false;
        }
    }

    public function requireAvailable(): void
    {
        if (!$this->available()) throw new ApiError(503, 'not_available', 'The asset registry is not set up yet (migration 2026-10-02-assets).');
    }

    // ── Reads ─────────────────────────────────────────────────────────────────────────────────────────────

    /** @return array<string, mixed>|null */
    public function asset(string $id): ?array
    {
        return $this->db->one('SELECT * FROM assets WHERE id = ?', [$id]);
    }

    /** @return array<string, mixed>|null */
    public function bySlug(string $slug): ?array
    {
        return $this->db->one('SELECT * FROM assets WHERE slug = ?', [$slug]);
    }

    /** @return array<string, mixed>|null the current version */
    public function currentVersion(string $assetId): ?array
    {
        return $this->db->one("SELECT * FROM asset_versions WHERE asset_id = ? AND status = 'current' ORDER BY number DESC LIMIT 1", [$assetId]);
    }

    /** @return list<array<string, mixed>> */
    public function versions(string $assetId): array
    {
        return $this->db->all('SELECT * FROM asset_versions WHERE asset_id = ? ORDER BY number DESC', [$assetId]);
    }

    /** @return list<array<string, mixed>> */
    public function renditions(int $versionId): array
    {
        return $this->db->all('SELECT * FROM asset_renditions WHERE version_id = ? ORDER BY path', [$versionId]);
    }

    /**
     * The asset + current rendition a path belongs to (only current versions count)
     * @return array{asset: array<string, mixed>, rendition: array<string, mixed>}|null
     */
    public function byPath(string $path): ?array
    {
        $row = $this->db->one("SELECT r.id AS rid FROM asset_renditions r JOIN asset_versions v ON v.id = r.version_id
            WHERE r.path = ? AND v.status = 'current' ORDER BY r.id DESC LIMIT 1", [$path]);
        if (!$row) return null;
        $rendition = $this->db->one('SELECT * FROM asset_renditions WHERE id = ?', [(int) $row['rid']]);
        $asset = $rendition ? $this->asset((string) $rendition['asset_id']) : null;
        return $asset && $rendition ? ['asset' => $asset, 'rendition' => $rendition] : null;
    }

    /** @return list<string> every path of the current versions (the files content may reference) */
    public function currentPaths(?string $assetId = null): array
    {
        $sql = "SELECT r.path FROM asset_renditions r JOIN asset_versions v ON v.id = r.version_id WHERE v.status = 'current'";
        $rows = $assetId === null ? $this->db->all($sql) : $this->db->all($sql . ' AND r.asset_id = ?', [$assetId]);
        return array_values(array_unique(array_map(fn(array $r) => (string) $r['path'], $rows)));
    }

    // ── Writes ────────────────────────────────────────────────────────────────────────────────────────────

    /** @param list<string> $tags */
    public function create(string $slug, string $kind, string $title = '', array $tags = [], ?string $by = null): string
    {
        if (!in_array($kind, AssetPaths::KINDS, true)) $kind = 'other';
        $id = Ids::uuid7();
        $now = $this->clock->utc();
        $this->db->run('INSERT INTO assets (id, slug, kind, folder, title, tags, status, created_at, updated_at, created_by) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)',
            [$id, $slug, $kind, AssetPaths::folder($slug), $title !== '' ? $title : self::titleOf($slug), self::encodeTags($tags), 'active', $now, $now, $by]);
        return $id;
    }

    /** "catpope/videos/s1-trailer-loop" → "s1-trailer-loop" */
    public static function titleOf(string $slug): string
    {
        $pos = strrpos($slug, '/');
        return $pos === false ? $slug : substr($slug, $pos + 1);
    }

    /** @param list<string> $tags */
    public static function encodeTags(array $tags): string
    {
        $clean = [];
        foreach ($tags as $tag) {
            $tag = strtolower(trim((string) $tag));
            if ($tag !== '' && strlen($tag) <= 40 && preg_match('/^[a-z0-9 _.-]+$/', $tag)) $clean[$tag] = true;
        }
        return (string) json_encode(array_keys($clean));
    }

    /**
     * A new current version (the previous current one becomes `archived`)
     * @param array{width?: ?int, height?: ?int, duration?: ?float} $probe
     */
    public function addVersion(string $assetId, StoredFile $file, string $storage, array $probe = [], ?string $by = null, ?string $note = null, ?string $createdAt = null): int
    {
        $number = (int) $this->db->value('SELECT COALESCE(MAX(number), 0) FROM asset_versions WHERE asset_id = ?', [$assetId]) + 1;
        $this->db->run("UPDATE asset_versions SET status = 'archived' WHERE asset_id = ? AND status = 'current'", [$assetId]);
        $this->db->run('INSERT INTO asset_versions (asset_id, number, path, storage, size, sha256, mime, width, height, duration, status, note, created_at, created_by) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)', [
            $assetId, $number, $file->key, $storage, $file->size, $file->sha256, $file->mime, $probe['width'] ?? null, $probe['height'] ?? null, $probe['duration'] ?? null,
            'current', $note, $createdAt ?? $this->clock->utc(), $by,
        ]);
        // read the id before touch(): after an UPDATE, MariaDB reports 0
        $id = $this->db->lastInsertId();
        $this->touch($assetId);
        return $id;
    }

    /** @param array{width?: ?int, height?: ?int, duration?: ?float, bitrate?: ?int} $probe */
    public function addRendition(string $assetId, int $versionId, string $label, ?string $quality, StoredFile $file, array $probe = [], string $origin = 'imported'): int
    {
        $this->db->run('INSERT INTO asset_renditions (asset_id, version_id, label, quality, path, size, sha256, mime, width, height, duration, bitrate, origin, modified_at, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)', [
            $assetId, $versionId, $label, $quality, $file->key, $file->size, $file->sha256, $file->mime, $probe['width'] ?? null, $probe['height'] ?? null,
            $probe['duration'] ?? null, $probe['bitrate'] ?? null, $origin, $file->modified !== '' ? gmdate('Y-m-d H:i:s', (int) strtotime($file->modified)) : null, $this->clock->utc(),
        ]);
        return $this->db->lastInsertId();
    }

    /** @param array{width?: ?int, height?: ?int, duration?: ?float, bitrate?: ?int} $probe */
    public function updateRendition(int $id, StoredFile $file, array $probe = []): void
    {
        $this->db->run('UPDATE asset_renditions SET size = ?, sha256 = ?, mime = ?, width = COALESCE(?, width), height = COALESCE(?, height), duration = COALESCE(?, duration), bitrate = COALESCE(?, bitrate), modified_at = ? WHERE id = ?', [
            $file->size, $file->sha256, $file->mime, $probe['width'] ?? null, $probe['height'] ?? null, $probe['duration'] ?? null, $probe['bitrate'] ?? null,
            $file->modified !== '' ? gmdate('Y-m-d H:i:s', (int) strtotime($file->modified)) : null, $id,
        ]);
    }

    public function removeRendition(int $id): void
    {
        $this->db->run('DELETE FROM asset_renditions WHERE id = ?', [$id]);
    }

    public function touch(string $assetId): void
    {
        $this->db->run('UPDATE assets SET updated_at = ? WHERE id = ?', [$this->clock->utc(), $assetId]);
    }

    /**
     * PATCH title / tags
     * @param array<string, mixed> $body
     * @return array<string, mixed>
     */
    public function updateMeta(string $id, array $body): array
    {
        $asset = $this->asset($id) ?? throw new ApiError(404, 'not_found', 'Unknown asset.');
        $title = array_key_exists('title', $body) ? trim((string) $body['title']) : (string) $asset['title'];
        if ($title === '' || mb_strlen($title) > 200) throw new ApiError(422, 'invalid_title', 'The title must be 1–200 characters.');
        $tags = self::tags($asset);
        if (array_key_exists('tags', $body)) {
            if (!is_array($body['tags'])) throw new ApiError(422, 'invalid_tags', 'tags must be a list.');
            $tags = [];
            foreach ($body['tags'] as $tag) if (is_scalar($tag)) $tags[] = (string) $tag;
        }
        $this->db->run('UPDATE assets SET title = ?, tags = ?, updated_at = ? WHERE id = ?', [$title, self::encodeTags($tags), $this->clock->utc(), $id]);
        return $this->asset($id) ?? [];
    }

    /**
     * @param array<string, mixed> $asset
     * @return list<string>
     */
    public static function tags(array $asset): array
    {
        $tags = json_decode((string) ($asset['tags'] ?? '[]'), true);
        return is_array($tags) ? array_values(array_map('strval', $tags)) : [];
    }

    // ── Lists for the UI ──────────────────────────────────────────────────────────────────────────────────

    /**
     * GET /assets?q=&folder=&kind=&tag=&state=used|unused|missing&page=&page_size=&sort=
     * One row per asset: its current version's main file (the preview), the renditions, sizes and usage counts.
     * @param array<mixed> $query
     * @return array<string, mixed>
     */
    public function list(array $query): array
    {
        $list = ListQuery::from($query, ['slug', 'updated', 'size', 'usages', 'kind'], 'slug', ['folder', 'kind', 'tag', 'state'], 60, 500);
        $where = ["a.status = 'active'"];
        $params = [];
        if ($list->q !== '') {
            $where[] = "(a.slug LIKE ? ESCAPE '!' OR a.title LIKE ? ESCAPE '!' OR a.tags LIKE ? ESCAPE '!')";
            $like = Database::likeContains($list->q);
            array_push($params, $like, $like, $like);
        }
        if (($folder = $list->filter('folder')) !== null) {
            $where[] = 'a.folder = ?';
            $params[] = $folder;
        }
        if (($kind = $list->filter('kind')) !== null) {
            if (!in_array($kind, AssetPaths::KINDS, true)) throw new ApiError(400, 'invalid_kind', 'Unknown kind.', ['kinds' => AssetPaths::KINDS]);
            $where[] = 'a.kind = ?';
            $params[] = $kind;
        }
        if (($tag = $list->filter('tag')) !== null) {
            $where[] = "a.tags LIKE ? ESCAPE '!'";
            $params[] = Database::likeContains('"' . strtolower($tag) . '"');
        }
        $usageCount = "(SELECT COUNT(*) FROM asset_usages u WHERE u.path IN (SELECT r2.path FROM asset_renditions r2 WHERE r2.version_id = v.id))";
        $state = $list->filter('state');
        if ($state === 'used') $where[] = "$usageCount > 0";
        elseif ($state === 'unused') $where[] = "$usageCount = 0";
        elseif ($state !== null) throw new ApiError(400, 'invalid_state', 'state is used | unused (missing files: GET /assets/missing).');

        $sql = "FROM assets a JOIN asset_versions v ON v.asset_id = a.id AND v.status = 'current' WHERE " . implode(' AND ', $where);
        $total = (int) $this->db->value("SELECT COUNT(*) $sql", $params);
        $order = match ($list->sortField) {
            'updated' => $list->orderBy('a.updated_at', 'a.slug'),
            'size' => $list->orderBy('total_size', 'a.slug'),
            'usages' => $list->orderBy('usages', 'a.slug'),
            'kind' => $list->orderBy('a.kind', 'a.slug'),
            default => $list->orderBy('a.slug', 'a.id'),
        };
        $rows = $this->db->all("SELECT a.*, v.id AS version_id, v.number AS version, v.path AS main_path, v.width, v.height, v.duration, v.size AS main_size,
                (SELECT COALESCE(SUM(r.size), 0) FROM asset_renditions r WHERE r.version_id = v.id) AS total_size,
                $usageCount AS usages
            $sql $order LIMIT {$list->pageSize} OFFSET {$list->offset()}", $params);
        $renditions = $this->renditionsOf(array_map(fn(array $r) => (int) $r['version_id'], $rows));
        $items = array_map(fn(array $r) => $this->summary($r, $renditions[(int) $r['version_id']] ?? []), $rows);
        return ['status' => 'success'] + $list->result($items, $total) + ['folders' => $this->folders()];
    }

    /**
     * Counts and bytes of the registry (current versions) per kind
     * @return array{assets: int, files: int, bytes: int, kinds: array<string, array{assets: int, files: int, bytes: int}>}
     */
    public function totals(): array
    {
        $kinds = [];
        $all = ['assets' => 0, 'files' => 0, 'bytes' => 0];
        foreach ($this->db->all("SELECT a.kind, COUNT(DISTINCT a.id) AS assets, COUNT(r.id) AS files, COALESCE(SUM(r.size), 0) AS bytes FROM assets a
                JOIN asset_versions v ON v.asset_id = a.id AND v.status = 'current' LEFT JOIN asset_renditions r ON r.version_id = v.id
                WHERE a.status = 'active' GROUP BY a.kind ORDER BY a.kind") as $r) {
            $row = ['assets' => (int) $r['assets'], 'files' => (int) $r['files'], 'bytes' => (int) $r['bytes']];
            $kinds[(string) $r['kind']] = $row;
            foreach ($row as $k => $v) $all[$k] += $v;
        }
        return $all + ['kinds' => $kinds];
    }

    /** @return list<array{folder: ?string, assets: int}> */
    public function folders(): array
    {
        return array_map(fn(array $r) => ['folder' => $r['folder'], 'assets' => (int) $r['n']],
            $this->db->all("SELECT folder, COUNT(*) AS n FROM assets WHERE status = 'active' GROUP BY folder ORDER BY folder"));
    }

    /**
     * @param list<int> $versionIds
     * @return array<int, list<array<string, mixed>>>
     */
    private function renditionsOf(array $versionIds): array
    {
        if (!$versionIds) return [];
        $marks = implode(',', array_fill(0, count($versionIds), '?'));
        $out = [];
        foreach ($this->db->all("SELECT * FROM asset_renditions WHERE version_id IN ($marks) ORDER BY path", $versionIds) as $r) {
            $out[(int) $r['version_id']][] = $r;
        }
        return $out;
    }

    /**
     * @param array<string, mixed> $row
     * @param list<array<string, mixed>> $renditions
     * @return array<string, mixed>
     */
    private function summary(array $row, array $renditions): array
    {
        return [
            'id' => $row['id'],
            'slug' => $row['slug'],
            'kind' => $row['kind'],
            'folder' => $row['folder'],
            'title' => $row['title'],
            'tags' => self::tags($row),
            'version' => (int) $row['version'],
            'path' => $row['main_path'],
            'width' => $row['width'] !== null ? (int) $row['width'] : null,
            'height' => $row['height'] !== null ? (int) $row['height'] : null,
            'duration' => $row['duration'] !== null ? (float) $row['duration'] : null,
            'size' => (int) $row['total_size'],
            'usages' => (int) $row['usages'],
            'updated_at' => Time::iso($row['updated_at']),
            'renditions' => array_map([self::class, 'rendition'], $renditions),
        ];
    }

    /**
     * @param array<string, mixed> $r
     * @return array<string, mixed>
     */
    public static function rendition(array $r): array
    {
        return [
            'id' => (int) $r['id'],
            'label' => $r['label'],
            'quality' => $r['quality'],
            'path' => $r['path'],
            'size' => (int) $r['size'],
            'mime' => $r['mime'],
            'width' => $r['width'] !== null ? (int) $r['width'] : null,
            'height' => $r['height'] !== null ? (int) $r['height'] : null,
            'duration' => $r['duration'] !== null ? (float) $r['duration'] : null,
            'origin' => $r['origin'],
        ];
    }

    /**
     * GET /assets/{id}: the asset, every version with its renditions, the usages of the current files
     * @return array<string, mixed>
     */
    public function detail(string $id): array
    {
        $asset = $this->asset($id) ?? throw new ApiError(404, 'not_found', 'Unknown asset.');
        $versions = [];
        foreach ($this->versions($id) as $v) {
            $versions[] = [
                'id' => (int) $v['id'],
                'number' => (int) $v['number'],
                'status' => $v['status'],
                'path' => $v['path'],
                'storage' => $v['storage'],
                'size' => (int) $v['size'],
                'sha256' => $v['sha256'],
                'mime' => $v['mime'],
                'width' => $v['width'] !== null ? (int) $v['width'] : null,
                'height' => $v['height'] !== null ? (int) $v['height'] : null,
                'duration' => $v['duration'] !== null ? (float) $v['duration'] : null,
                'archive_path' => $v['archive_path'],
                'note' => $v['note'],
                'created_at' => Time::iso($v['created_at']),
                'created_by' => $v['created_by'],
                'renditions' => array_map([self::class, 'rendition'], $this->renditions((int) $v['id'])),
            ];
        }
        $paths = $this->currentPaths($id);
        $jobs = $this->db->all('SELECT id, kind, status, created_at, finished_at FROM asset_jobs WHERE asset_id = ? ORDER BY id DESC LIMIT 20', [$id]);
        return [
            'status' => 'success',
            'asset' => [
                'id' => $asset['id'], 'slug' => $asset['slug'], 'kind' => $asset['kind'], 'folder' => $asset['folder'], 'title' => $asset['title'],
                'tags' => self::tags($asset), 'status' => $asset['status'], 'created_at' => Time::iso($asset['created_at']), 'updated_at' => Time::iso($asset['updated_at']),
            ],
            'versions' => $versions,
            'usages' => $this->usagesOf($paths),
            'jobs' => array_map(fn(array $j) => ['id' => (int) $j['id'], 'kind' => $j['kind'], 'status' => $j['status'], 'created_at' => Time::iso($j['created_at']), 'finished_at' => Time::iso($j['finished_at'])], $jobs),
        ];
    }

    /**
     * Where these paths are referenced (the delete check, the detail page, the picker's "used by")
     * @param list<string> $paths
     * @return list<array<string, mixed>>
     */
    public function usagesOf(array $paths): array
    {
        if (!$paths) return [];
        $marks = implode(',', array_fill(0, count($paths), '?'));
        return array_map(fn(array $u) => [
            'path' => $u['path'], 'source' => $u['source'], 'ref' => $u['ref'], 'field' => $u['field'], 'state' => $u['state'],
            'toon' => $u['toon'], 'raw' => $u['raw'],
        ], $this->db->all("SELECT * FROM asset_usages WHERE path IN ($marks) ORDER BY source, ref, field, state", $paths));
    }
}
