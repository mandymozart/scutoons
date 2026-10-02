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
use Scutoons\Http\Middleware\JsonBody;
use Scutoons\Infrastructure\AssetStorage\AssetStorages;
use Scutoons\Infrastructure\AssetStorage\StorageKey;

/**
 * The asset manager (agents/ASSETS.md "API", docs/API.md "Assets"). Content editors (creator, admin) browse, look up,
 * upload new files and edit titles / tags – the creator's asset picker. Replacing, renaming, relinking, deleting,
 * scanning, jobs and storage status are admin only (admin app → Assets).
 *   GET  /api/assets?q=&folder=&kind=&tag=&state=used|unused&page=&page_size=&sort=   list (current versions)
 *   GET  /api/assets/lookup?path=                     the asset a path belongs to (null when unregistered)
 *   GET  /api/assets/{id}                             detail: versions, renditions, usages, jobs
 *   PUT  /api/assets/{id}                             { title?, tags? }
 *   PUT  /api/assets/upload?path=&title=               raw body = the file → 201 (409 exists)
 *   GET  /api/assets/missing · /api/assets/orphans    references to unknown files · files nothing references
 *   ── admin ──
 *   PUT  /api/admin/assets/{id}/file?path=&note=       raw body: replace that file in place (new version, purge)
 *   POST /api/admin/assets/{id}/rename                 { slug, publish? }
 *   POST /api/admin/assets/relink                      { from, to, publish? }
 *   GET  /api/admin/assets/{id}/delete-check · DELETE /api/admin/assets/{id}?force=1
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
        return Json::response($response, 200, $result + ['cdn' => $this->storages->publicBase()]);
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
        return Json::response($response, 200, $this->registry->detail($args['id']) + ['cdn' => $this->storages->publicBase()]);
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
        return Json::response($response, 200, $this->service->relink($user, (string) ($body['from'] ?? ''), (string) ($body['to'] ?? ''), ($body['publish'] ?? false) === true));
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
            'public_base' => $this->storages->publicBase(),
            'totals' => $totals,
            'bandwidth' => $stats,
            'bandwidth_error' => $statsError,
        ]);
    }

    /** Streams the raw request body into a temp file (never the whole video in memory); 413 above the limit */
    private function bodyToFile(Request $request): string
    {
        $max = $this->service->maxUploadBytes();
        $declared = (int) $request->getHeaderLine('Content-Length');
        if ($declared > $max) throw new ApiError(413, 'file_too_large', "The file is larger than $max bytes.");
        $tmp = (string) tempnam(sys_get_temp_dir(), 'asset-upload-');
        $out = fopen($tmp, 'wb');
        if ($out === false) throw new ApiError(500, 'upload_failed', 'No temp file.');
        $raw = $request->getAttribute(JsonBody::ATTRIBUTE);
        $written = 0;
        if (is_string($raw)) {
            $written = (int) fwrite($out, $raw);
        } else {
            $body = $request->getBody();
            if ($body->isSeekable()) $body->rewind();
            while (!$body->eof()) {
                $chunk = $body->read(1048576);
                $written += (int) fwrite($out, $chunk);
                if ($written > $max) {
                    fclose($out);
                    @unlink($tmp);
                    throw new ApiError(413, 'file_too_large', "The file is larger than $max bytes.");
                }
            }
        }
        fclose($out);
        if ($written === 0) {
            @unlink($tmp);
            throw new ApiError(422, 'empty_file', 'The request carried no file.');
        }
        return $tmp;
    }
}
