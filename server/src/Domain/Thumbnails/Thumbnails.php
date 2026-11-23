<?php
declare(strict_types=1);

namespace Scutoons\Domain\Thumbnails;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Assets\AssetJobs;
use Scutoons\Domain\Assets\AssetPaths;
use Scutoons\Domain\Assets\AssetRegistry;
use Scutoons\Infrastructure\AssetStorage\AssetStorages;
use Scutoons\Infrastructure\AssetStorage\StoredFile;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Time;

/**
 * One standard for thumbnails (#842, server/docs/API.md "Thumbnails"): the deck editor's still made shared – a 264 × 168
 * WebP (STILL in deck/editor/src/library.rs, centre-cut), keyed by what it shows:
 *   asset:<id>         the asset's `thumb` rendition `<slug>.thumb.webp` on its current version; stamp = the version number.
 *                      Images and videos are made by the server (an AssetJobs `thumb` job, queued on every new version);
 *                      what only a renderer can draw (3D models …) is uploaded by the creator or the deck.
 *   workspace:<id>     a member's snapshot of a creator workspace – private: listed only to that member, a random file name.
 *   <address>          an item, a scene, a character, a chapter, a toon … (`item:camera`, A14) – shared, uploaded by whoever renders it.
 * The stamp says what a still was taken from; a still whose stamp no longer matches (or a row marked stale) is made again.
 * Files live in the primary asset storage (local in dev, bunny in production); non-asset stills under `.thumbs/` (never
 * registered as assets), each upload a new name, the old file removed.
 */
final class Thumbnails
{
    public const WIDTH = 264;
    public const HEIGHT = 168;
    public const MAX_BYTES = 262_144;
    public const MAX_KEYS = 100;
    public const PREFIX = '.thumbs/';
    private const ASSET = '/^asset:([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})$/';
    private const WORKSPACE = '/^workspace:([a-z][a-z0-9_-]{0,47})$/';
    private const ADDRESS = '#^([a-z][a-z0-9_]{0,31}):[A-Za-z0-9_.:@/-]{1,160}$#';

    public function __construct(
        private readonly ThumbnailRows $rows,
        private readonly AssetRegistry $registry,
        private readonly AssetStorages $storages,
        private readonly AssetJobs $jobs,
        private readonly Database $db,
    ) {
    }

    /** 'asset' | 'workspace' | 'address'; 400 bad_key otherwise */
    public static function kindOf(string $key): string
    {
        if (preg_match(self::ASSET, $key) === 1) return 'asset';
        if (preg_match(self::WORKSPACE, $key) === 1) return 'workspace';
        if (preg_match(self::ADDRESS, $key, $m) === 1 && !in_array($m[1], ['asset', 'workspace'], true) && !str_contains($key, '..')) return 'address';
        throw new ApiError(400, 'bad_key', 'A thumbnail key: asset:<id>, workspace:<id> or an address (item:camera).', ['key' => mb_substr($key, 0, 200)]);
    }

    /**
     * GET /api/thumbs?keys=a,b: the stored stills of these keys (missing ones are left out). Workspace keys answer only
     * the member's own snapshots.
     * @param list<string> $keys
     * @return array{available: bool, thumbs: array<string, array{url: string, stamp: string, stale: bool, origin: string, taken_at: ?string}>}
     */
    public function lookup(array $keys, ?string $userId): array
    {
        $keys = array_values(array_unique(array_filter(array_map('trim', $keys), static fn(string $k) => $k !== '')));
        if (count($keys) > self::MAX_KEYS) throw new ApiError(400, 'too_many_keys', 'At most ' . self::MAX_KEYS . ' keys per lookup.', ['max' => self::MAX_KEYS]);
        if (!$this->rows->available()) return ['available' => false, 'thumbs' => []];
        $shared = [];
        $own = [];
        $assets = [];
        foreach ($keys as $key) {
            $kind = self::kindOf($key);
            if ($kind === 'workspace') $own[] = $key;
            else $shared[] = $key;
            if ($kind === 'asset') $assets[] = substr($key, 6);
        }
        $versions = $this->currentVersions($assets);
        $storage = $this->storages->primary();
        $out = [];
        foreach ($this->rows->of($shared, $own, $userId) as $key => $r) {
            $stamp = (string) $r['stamp'];
            $stale = (int) $r['stale'] === 1;
            if (str_starts_with($key, 'asset:')) $stale = $stale || ($versions[substr($key, 6)] ?? null) !== $stamp;
            $out[$key] = [
                'url' => $storage->publicUrl((string) $r['path']) . '?v=' . substr(md5($stamp . '|' . (string) $r['taken_at']), 0, 10),
                'stamp' => $stamp,
                'stale' => $stale,
                'origin' => (string) $r['origin'],
                'taken_at' => Time::iso($r['taken_at']),
            ];
        }
        return ['available' => true, 'thumbs' => $out];
    }

    /**
     * PUT /api/thumbs?key=&stamp=: a still for the key (the caller has checked the rights: a content editor for shared keys,
     * any member for their own workspace). The file must be a WebP of exactly WIDTH × HEIGHT.
     * @return array{key: string, url: string, stamp: string, stale: bool, origin: string, taken_at: ?string}
     */
    public function put(string $userId, string $key, string $file, ?string $stamp): array
    {
        $kind = self::kindOf($key);
        if (!$this->rows->available()) throw new ApiError(503, 'not_available', 'Thumbnails are not set up yet.');
        self::checkWebp($file);
        $size = (int) filesize($file);
        $owner = $kind === 'workspace' ? $userId : '';
        $storage = $this->storages->primary();
        $was = $this->rows->one($owner, $key);
        if ($kind === 'asset') {
            $assetId = substr($key, 6);
            $asset = $this->registry->asset($assetId) ?? throw new ApiError(404, 'not_found', 'Unknown asset.');
            if ($asset['status'] !== 'active') throw new ApiError(409, 'deleted', 'The asset was deleted.');
            $version = $this->registry->currentVersion($assetId) ?? throw new ApiError(409, 'no_version', 'The asset has no current version.');
            $path = AssetJobs::outputPath((string) $asset['slug'], 'thumb', 'webp');
            $existed = $storage->stat($path) !== null;
            $stored = $storage->upload($path, $file, 'image/webp');
            if ($existed) $storage->purge($path);
            $this->assetRendition($assetId, (int) $version['id'], $stored, $file);
            $stamp = $stamp !== null && $stamp !== '' ? $stamp : (string) $version['number'];
        } else {
            $path = self::PREFIX . ($kind === 'workspace' ? 'members/' : 'shared/') . bin2hex(random_bytes(12)) . '.webp';
            $storage->upload($path, $file, 'image/webp');
            $stamp ??= '';
        }
        $this->rows->record($owner, $key, $path, $stamp, 'client', $size, $userId);
        if ($was !== null && (string) $was['path'] !== $path && str_starts_with((string) $was['path'], self::PREFIX)) {
            try {
                $storage->delete((string) $was['path']);
            } catch (\Throwable) {
                // a file left behind is harmless; the row points at the new one
            }
        }
        $row = $this->lookup([$key], $userId)['thumbs'][$key] ?? throw new ApiError(500, 'not_saved', 'The thumbnail was not saved.');
        return ['key' => $key] + $row;
    }

    /**
     * A new version of an asset was written (upload of a new asset, replace of its source): its still is stale, and for
     * the kinds the server can draw a `thumb` job is queued (once).
     */
    public function assetChanged(string $assetId, string $kind, string $source, ?string $by = null): void
    {
        $this->rows->markStale('asset:' . $assetId);
        if (!AssetJobs::thumbable($kind, $source)) return;
        try {
            $queued = $this->db->value("SELECT id FROM asset_jobs WHERE asset_id = ? AND kind = 'thumb' AND status = 'queued' LIMIT 1", [$assetId]);
            if ($queued === null) $this->jobs->enqueue($assetId, 'thumb', [], $by);
        } catch (\PDOException) {
            // no job table yet: the still waits as stale
        }
    }

    /**
     * `assets:jobs --thumbs`: a `thumb` job for every image / video (not a map tile) whose still is missing or stale and
     * none is queued – the stills of what was there before the standard. Returns how many were queued.
     */
    public function queueMissing(int $limit = 500): int
    {
        if (!$this->rows->available()) return 0;
        $rows = $this->db->all(
            "SELECT a.id, a.kind, v.path, v.number FROM assets a JOIN asset_versions v ON v.asset_id = a.id AND v.status = 'current'
             WHERE a.status = 'active' AND a.kind IN ('image', 'video') AND a.slug NOT REGEXP ?
               AND NOT EXISTS (SELECT 1 FROM asset_jobs j WHERE j.asset_id = a.id AND j.kind = 'thumb' AND j.status = 'queued')
             ORDER BY a.updated_at DESC",
            [AssetPaths::TILE_SQL]
        );
        $queued = 0;
        foreach (array_chunk($rows, 200) as $chunk) {
            $have = $this->rows->of(array_map(static fn(array $r) => 'asset:' . $r['id'], $chunk), [], null);
            foreach ($chunk as $r) {
                if ($queued >= $limit) return $queued;
                $thumb = $have['asset:' . $r['id']] ?? null;
                if ($thumb !== null && (int) $thumb['stale'] === 0 && (string) $thumb['stamp'] === (string) $r['number']) continue;
                if (!AssetJobs::thumbable((string) $r['kind'], (string) $r['path'])) continue;
                $this->jobs->enqueue((string) $r['id'], 'thumb', [], 'cli');
                $queued++;
            }
        }
        return $queued;
    }

    /** The `thumb` rendition of the asset's current version: added, or updated in place */
    private function assetRendition(string $assetId, int $versionId, StoredFile $stored, string $file): void
    {
        $stored = new StoredFile($stored->key, $stored->size, gmdate('c'), hash_file('sha256', $file) ?: null, 'image/webp');
        $probe = ['width' => self::WIDTH, 'height' => self::HEIGHT];
        foreach ($this->registry->renditions($versionId) as $r) {
            if ((string) $r['path'] === $stored->key) {
                $this->registry->updateRendition((int) $r['id'], $stored, $probe);
                return;
            }
        }
        $this->registry->addRendition($assetId, $versionId, 'thumb', null, $stored, $probe, 'generated');
    }

    /**
     * @param list<string> $assetIds
     * @return array<string, string> asset id → its current version number
     */
    private function currentVersions(array $assetIds): array
    {
        if ($assetIds === []) return [];
        $rows = $this->db->all("SELECT asset_id, MAX(number) AS n FROM asset_versions WHERE status = 'current' AND asset_id IN (" . implode(',', array_fill(0, count($assetIds), '?')) . ') GROUP BY asset_id', $assetIds);
        $out = [];
        foreach ($rows as $r) $out[(string) $r['asset_id']] = (string) $r['n'];
        return $out;
    }

    /** 400 bad_thumb unless the file is a WebP of exactly WIDTH × HEIGHT; 413 above MAX_BYTES */
    public static function checkWebp(string $file): void
    {
        $size = (int) @filesize($file);
        if ($size > self::MAX_BYTES) throw new ApiError(413, 'thumb_too_large', 'A thumbnail is at most ' . self::MAX_BYTES . ' bytes.', ['max' => self::MAX_BYTES]);
        $head = (string) @file_get_contents($file, false, null, 0, 32);
        $dims = self::webpSize($head);
        if ($dims === null) throw new ApiError(400, 'bad_thumb', 'A thumbnail is a WebP.');
        if ($dims !== [self::WIDTH, self::HEIGHT]) throw new ApiError(400, 'bad_thumb', 'A thumbnail is ' . self::WIDTH . ' × ' . self::HEIGHT . '.', ['width' => $dims[0], 'height' => $dims[1]]);
    }

    /** @return array{0: int, 1: int}|null the WebP's canvas size from its first 30 bytes (VP8, VP8L, VP8X) */
    public static function webpSize(string $head): ?array
    {
        if (strlen($head) < 30 || substr($head, 0, 4) !== 'RIFF' || substr($head, 8, 4) !== 'WEBP') return null;
        $b = static fn(int $i): int => ord($head[$i]);
        return match (substr($head, 12, 4)) {
            'VP8 ' => substr($head, 23, 3) === "\x9d\x01\x2a" ? [($b(26) | $b(27) << 8) & 0x3fff, ($b(28) | $b(29) << 8) & 0x3fff] : null,
            'VP8L' => $b(20) === 0x2f ? [1 + ($b(21) | ($b(22) & 0x3f) << 8), 1 + (($b(22) >> 6) | $b(23) << 2 | ($b(24) & 0x0f) << 10)] : null,
            'VP8X' => [1 + ($b(24) | $b(25) << 8 | $b(26) << 16), 1 + ($b(27) | $b(28) << 8 | $b(29) << 16)],
            default => null,
        };
    }
}
