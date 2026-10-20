<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Assets\AssetImporter;
use Scutoons\Domain\Assets\AssetJobs;
use Scutoons\Domain\Assets\AssetRegistry;
use Scutoons\Domain\Assets\AssetService;
use Scutoons\Domain\Assets\AssetUsages;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Json;
use Scutoons\Http\RawUpload;
use Scutoons\Http\Middleware\JsonBody;
use Scutoons\Infrastructure\AssetStorage\AssetStorages;
use Scutoons\Infrastructure\AssetStorage\StorageKey;

/**
 * The asset manager (agents/ASSETS.md "API", docs/API.md "Assets"). Content editors (creator, admin) browse, look up,
 * upload new files and edit titles / tags – the creator's asset picker. Replacing, renaming, relinking, deleting,
 * scanning, jobs and storage status are admin only (admin app → Assets).
 *   GET  /api/assets?q=&folder=&kind=&tag=&state=used|unused&tiles=hide|only|all&page=&page_size=&sort=   list (map tiles hidden by default; kind=shape = the vector library)
 *   GET  /api/assets/lookup?path=                     the asset a path belongs to (null when unregistered)
 *   GET  /api/assets/{id}                             detail: versions, renditions, usages, jobs
 *   PUT  /api/assets/{id}                             { title?, tags?, shape? } (shape: { tile, mask, colors, source?, derived? } | null)
 *   PUT  /api/assets/upload?path=&title=               raw body = the file → 201 (409 exists)
 *   GET  /api/assets/missing · /api/assets/orphans    references to unknown files · files nothing references
 *   ── admin ──
 *   PUT  /api/admin/assets/{id}/file?path=&note=       raw body: replace that file in place (new version, purge)
 *   POST /api/admin/assets/{id}/rename                 { slug, publish? }
 *   POST /api/admin/assets/relink                      { from, to, publish? } or { map: { old: new … }, publish? }
 *   GET  /api/admin/assets/{id}/delete-check · DELETE /api/admin/assets/{id}?force=1
 *   GET  /api/admin/assets/unused?q=&folder=&kind=&what=asset|files&tiles=all|hide|only&sort=&page=&fresh=1&all=1
 *   POST /api/admin/assets/cleanup { paths, rebuild? }   unused files (a tile set = one row) · delete them (in batches)
 *   POST /api/admin/assets/{id}/jobs                   { kind } → queued job · GET /api/admin/assets/jobs
 *   POST /api/admin/assets/scan                        import the storage + rebuild usages
 *   GET  /api/admin/assets/storage                     drivers, settings status (never secrets), bandwidth
 */
final class AssetsController
{
    public function __construct(
        private readonly AssetRegistry $registry,
        private readonly AssetService $service,
        private readonly AssetUsages $usages,
        private readonly AssetImporter $importer,
        private readonly AssetJobs $jobs,
        private readonly AssetStorages $storages,
        private readonly Roles $roles,
    ) {
    }

    /** @return array<string, mixed> */
    private function editor(Request $request): array
    {
        $user = $this->roles->requireContentEditor(CurrentSession::of($request)->user());
        CurrentSession::of($request)->rejectCsrf();
        $this->registry->requireAvailable();
        return $user;
    }

    /** @return array<string, mixed> */
    private function admin(Request $request): array
    {
        $user = $this->roles->require(CurrentSession::of($request)->requireUser(), Roles::ADMIN);
        $this->registry->requireAvailable();
        return $user;
    }

    public function list(Request $request, Response $response): Response
    {
        $this->editor($request);
        $result = $this->registry->list($request->getQueryParams());
        return Json::response($response, 200, $result + ['cdn' => $this->storages->primaryPublicBase()]);
    }

    public function lookup(Request $request, Response $response): Response
    {
        $this->editor($request);
        $path = (string) ($request->getQueryParams()['path'] ?? '');
        if (!StorageKey::valid($path)) throw new ApiError(400, 'invalid_path', 'path is required.');
        $found = $this->registry->byPath(StorageKey::normalize($path));
        return Json::response($response, 200, ['status' => 'success', 'asset' => $found ? $this->registry->detail((string) $found['asset']['id']) : null,
            'rendition' => $found ? AssetRegistry::rendition($found['rendition']) : null]);
    }

    /** @param array{id: string} $args */
    public function get(Request $request, Response $response, array $args): Response
    {
        $this->editor($request);
        return Json::response($response, 200, $this->registry->detail($args['id']) + ['cdn' => $this->storages->primaryPublicBase()]);
    }

    /** @param array{id: string} $args */
    public function update(Request $request, Response $response, array $args): Response
    {
        $this->editor($request);
        $this->registry->updateMeta($args['id'], JsonBody::of($request));
        return Json::response($response, 200, $this->registry->detail($args['id']));
    }

    public function upload(Request $request, Response $response): Response
    {
        $user = $this->editor($request);
        $query = $request->getQueryParams();
        $path = AssetService::targetPath((string) ($query['path'] ?? ''));
        $tags = isset($query['tags']) && is_string($query['tags']) ? array_values(array_filter(array_map('trim', explode(',', $query['tags'])))) : [];
        $file = $this->bodyToFile($request);
        try {
            $result = $this->service->upload($user, $file, $path, (string) ($query['title'] ?? ''), $tags);
        } finally {
            @unlink($file);
        }
        return Json::response($response, 201, $result);
    }

    /**
     * PUT /api/assets/protected?item=<item id>&file=musicxml|pdf – a score's file, or ?album=<album id>&file=track|download
     * – a physical release's audio (an mp3, the album zip; owner 2026-10-14: release audio, not game audio) – into the protected storage
     * (agents/research/SCORES.md §5; AssetStorages::protectedStorage), never the public one and not in the asset registry:
     * `scores/<item>/` or `albums/<album>/<first 12 of the sha-256>.<ext>` (a changed file gets a new name) → 201 { path }. The catalog
     * editor puts the path into the item's `score`; readers only ever get signed URLs (ProtectedFiles).
     * 400 invalid_item / invalid_file / wrong_type, 413 file_too_large, 503 not_configured.
     */
    public function uploadProtected(Request $request, Response $response): Response
    {
        $this->roles->requireContentEditor(CurrentSession::of($request)->user());
        CurrentSession::of($request)->rejectCsrf();
        $query = $request->getQueryParams();
        // a score's file (item=…&file=musicxml|pdf) or a physical release's audio (album=…&file=track|download)
        $album = is_string($query['album'] ?? null) ? $query['album'] : null;
        $owner = $album ?? (is_string($query['item'] ?? null) ? $query['item'] : '');
        if (!preg_match('/^[a-z0-9_]{1,100}$/', $owner)) throw new ApiError(400, $album !== null ? 'invalid_album' : 'invalid_item', $album !== null ? 'album: the album id.' : 'item: the catalog item id.');
        $kinds = $album !== null ? ['track', 'download'] : ['musicxml', 'pdf'];
        $kind = in_array($query['file'] ?? null, $kinds, true) ? (string) $query['file'] : throw new ApiError(400, 'invalid_file', 'file: ' . implode(' or ', $kinds) . '.');
        $storage = $this->storages->protectedStorage() ?? throw new ApiError(503, 'not_configured', 'The protected storage is not set up (BUNNY_PROTECTED_STORAGE_ZONE + _PASSWORD).');
        $file = $this->bodyToFile($request);
        try {
            $head = (string) file_get_contents($file, false, null, 0, 512);
            $ext = match (true) {
                $kind === 'pdf' && str_starts_with($head, '%PDF') => 'pdf',
                $kind === 'musicxml' && str_starts_with($head, "PK\x03\x04") => 'mxl',
                $kind === 'musicxml' && (str_contains($head, '<score-partwise') || str_contains($head, '<score-timewise') || str_contains($head, 'MusicXML')) => 'musicxml',
                $kind === 'track' && (str_starts_with($head, 'ID3') || (strlen($head) > 1 && ord($head[0]) === 0xFF && (ord($head[1]) & 0xE0) === 0xE0)) => 'mp3',
                $kind === 'download' && str_starts_with($head, "PK\x03\x04") => 'zip',
                default => throw new ApiError(400, 'wrong_type', match ($kind) { 'pdf' => 'That is not a PDF.', 'track' => 'That is not an MP3.', 'download' => 'That is not a ZIP.', default => 'That is not MusicXML (.musicxml / .mxl).' }),
            };
            $path = ($album !== null ? 'albums' : 'scores') . "/$owner/" . substr((string) hash_file('sha256', $file), 0, 12) . ".$ext";
            $storage->upload($path, $file, match ($ext) { 'pdf' => 'application/pdf', 'mxl' => 'application/vnd.recordare.musicxml', 'mp3' => 'audio/mpeg', 'zip' => 'application/zip', default => 'application/vnd.recordare.musicxml+xml' });
        } finally {
            @unlink($file);
        }
        return Json::response($response, 201, ['path' => $path]);
    }

    /**
     * New bytes for a file that is already there (owner 2026-10-03: "creators should be able to replace slice images" –
     * updating media meant renaming): the asset's `replace` by path – the old bytes are archived as a version, the CDN
     * purged. 404 when no asset has that file, 409 `unchanged` when the bytes are the same.
     */
    public function replaceAt(Request $request, Response $response): Response
    {
        $user = $this->editor($request);
        $query = $request->getQueryParams();
        $path = (string) ($query['path'] ?? '');
        if (!StorageKey::valid($path)) throw new ApiError(400, 'invalid_path', 'path is required.');
        $path = StorageKey::normalize($path);
        $found = $this->registry->byPath($path) ?? throw new ApiError(404, 'not_found', 'No asset has that file.');
        $file = $this->bodyToFile($request);
        try {
            $result = $this->service->replace($user, (string) $found['asset']['id'], $file, $path, isset($query['note']) && is_string($query['note']) ? mb_substr($query['note'], 0, 200) : null);
        } finally {
            @unlink($file);
        }
        return Json::response($response, 200, $result);
    }

    public function missing(Request $request, Response $response): Response
    {
        $this->editor($request);
        return Json::response($response, 200, ['status' => 'success', 'items' => $this->usages->missing()]);
    }

    public function orphans(Request $request, Response $response): Response
    {
        $this->editor($request);
        return Json::response($response, 200, ['status' => 'success', 'items' => $this->usages->orphans()]);
    }

    // ── admin ─────────────────────────────────────────────────────────────────────────────────────────────

    /** @param array{id: string} $args */
    public function replace(Request $request, Response $response, array $args): Response
    {
        $user = $this->admin($request);
        $query = $request->getQueryParams();
        $path = isset($query['path']) && is_string($query['path']) && $query['path'] !== '' ? StorageKey::normalize($query['path']) : null;
        $file = $this->bodyToFile($request);
        try {
            $result = $this->service->replace($user, $args['id'], $file, $path, isset($query['note']) && is_string($query['note']) ? mb_substr($query['note'], 0, 200) : null);
        } finally {
            @unlink($file);
        }
        return Json::response($response, 200, $result);
    }

    /** @param array{id: string} $args */
    public function rename(Request $request, Response $response, array $args): Response
    {
        $user = $this->admin($request);
        $body = JsonBody::of($request);
        return Json::response($response, 200, $this->service->rename($user, $args['id'], (string) ($body['slug'] ?? ''), ($body['publish'] ?? false) === true));
    }

    public function relink(Request $request, Response $response): Response
    {
        $user = $this->admin($request);
        $body = JsonBody::of($request);
        $publish = ($body['publish'] ?? false) === true;
        // `map` (old path → new path): several files in one pass – one draft per touched entity (BACKLOG 10.64)
        if (isset($body['map'])) {
            if (!is_array($body['map'])) throw new ApiError(400, 'invalid_map', 'map is an object: old path → new path.');
            return Json::response($response, 200, $this->service->relinkAll($user, $body['map'], $publish));
        }
        return Json::response($response, 200, $this->service->relink($user, (string) ($body['from'] ?? ''), (string) ($body['to'] ?? ''), $publish));
    }

    /** @param array{id: string} $args */
    public function deleteCheck(Request $request, Response $response, array $args): Response
    {
        $this->admin($request);
        return Json::response($response, 200, $this->service->deleteCheck($args['id']));
    }

    /** @param array{id: string} $args */
    public function delete(Request $request, Response $response, array $args): Response
    {
        $user = $this->admin($request);
        return Json::response($response, 200, $this->service->delete($user, $args['id'], ($request->getQueryParams()['force'] ?? '') === '1'));
    }

    public function unused(Request $request, Response $response): Response
    {
        $this->admin($request);
        return Json::response($response, 200, $this->service->unused($request->getQueryParams()));
    }

    public function cleanup(Request $request, Response $response): Response
    {
        $user = $this->admin($request);
        $body = JsonBody::of($request);
        $paths = $body['paths'] ?? null;
        if (!is_array($paths) || !$paths || count($paths) > 5000 || array_filter($paths, fn($p) => !is_string($p))) {
            throw new ApiError(422, 'invalid_paths', 'paths must be a non-empty list of CDN paths.');
        }
        // a bulk delete's later batches: `rebuild: false` (the first one rebuilt the usage index)
        return Json::response($response, 200, $this->service->cleanup($user, array_values($paths), ($body['rebuild'] ?? true) !== false));
    }

    /** @param array{id: string} $args */
    public function enqueue(Request $request, Response $response, array $args): Response
    {
        $user = $this->admin($request);
        $body = JsonBody::of($request);
        return Json::response($response, 201, ['status' => 'success', 'job' => $this->jobs->enqueue($args['id'], (string) ($body['kind'] ?? ''), is_array($body['params'] ?? null) ? $body['params'] : [], (string) $user['id'])]);
    }

    public function jobs(Request $request, Response $response): Response
    {
        $this->admin($request);
        return Json::response($response, 200, ['status' => 'success', 'items' => $this->jobs->recent(), 'kinds' => AssetJobs::KINDS]);
    }

    public function scan(Request $request, Response $response): Response
    {
        $this->admin($request);
        $stats = $this->importer->import($this->service->storage(), '', true, false);
        return Json::response($response, 200, ['status' => 'success', 'import' => $stats, 'usages' => $this->usages->rebuild()]);
    }

    public function storage(Request $request, Response $response): Response
    {
        $this->admin($request);
        $bunny = $this->storages->bunny();
        $stats = null;
        $statsError = null;
        if ($bunny && $bunny->hasApiKey() && ($request->getQueryParams()['stats'] ?? '') === '1') {
            try {
                $raw = $bunny->statistics(gmdate('Y-m-d', time() - 30 * 86400), gmdate('Y-m-d'));
                if ($raw) {
                    $chart = is_array($raw['BandwidthUsedChart'] ?? null) ? $raw['BandwidthUsedChart'] : [];
                    $stats = ['days' => 30, 'bandwidth' => (int) ($raw['TotalBandwidthUsed'] ?? 0), 'requests' => (int) ($raw['TotalRequestsServed'] ?? 0),
                        'cache_hit_rate' => (float) ($raw['CacheHitRate'] ?? 0), 'daily' => array_map(fn($k, $v) => ['day' => substr((string) $k, 0, 10), 'bytes' => (int) $v], array_keys($chart), $chart)];
                }
            } catch (\Throwable $error) {
                $statsError = $error->getMessage();
            }
        }
        $totals = $this->registry->totals();
        return Json::response($response, 200, [
            'status' => 'success',
            'primary' => $this->storages->primaryName(),
            'local' => ['where' => $this->storages->local()->describe()],
            'bunny' => $bunny ? ['where' => $bunny->describe(), 'can_write' => $bunny->canWrite(), 'api_key' => $bunny->hasApiKey(), 'pull_zone' => $bunny->pullZoneId()] : null,
            'settings' => $this->storages->configured(),
            'public_base' => $this->storages->primaryPublicBase(),
            'bunny_public_base' => $this->storages->publicBase(),
            'totals' => $totals,
            'bandwidth' => $stats,
            'bandwidth_error' => $statsError,
        ]);
    }

    /** Streams the raw request body into a temp file (never the whole video in memory); 413 above the limit */
    private function bodyToFile(Request $request): string
    {
        return RawUpload::toTempFile($request, $this->service->maxUploadBytes(), 'asset-upload-');
    }
}
