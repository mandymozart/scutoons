<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers\Admin;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\Admin\Batches;
use Scutoons\Domain\Admin\ListQuery;
use Scutoons\Domain\ApiError;
use Scutoons\Http\Download;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Storage\FileStore;

/**
 * /api/admin/batches and /api/admin/files – role admin (docs/API.md "Admin API"):
 *   GET  /batches                      list (page, page_size, sort, q, edition_id, complete)
 *   POST /batches                      { edition_id, count?, start_number?, force? } → new manifest (codes reserved, not inserted)
 *   GET  /batches/{filename}           the manifest
 *   PUT  /batches/{filename}           { manifest } → store progress
 *   GET  /batches/{filename}/zip       ZIP: manifest + SVG/PDF/signature of every finished code
 *   PUT  /files/{type}/{filename}      { doc } store an SVG / PDF (data URI) / JSON
 *   GET  /files/{type}/{filename}      the stored file
 */
final class BatchesController
{
    private const SORTABLE = ['created_at', 'filename', 'edition', 'total', 'done', 'errors'];

    public function __construct(
        private readonly Batches $batches,
        private readonly FileStore $files,
        private readonly Settings $settings,
    ) {
    }

    public function list(Request $request, Response $response): Response
    {
        $query = ListQuery::from($request->getQueryParams(), self::SORTABLE, '-created_at', ['edition_id', 'complete'], 50, $this->settings->int('ADMIN_MAX_PAGE_SIZE'));
        $rows = $this->batches->summaries();
        $rows = array_values(array_filter($rows, function (array $b) use ($query) {
            if ($query->q !== '' && stripos($b['filename'] . ' ' . $b['edition'], $query->q) === false) return false;
            if (($edition = $query->filter('edition_id')) !== null && $b['editionId'] !== $edition) return false;
            if (($complete = $query->filter('complete')) !== null && $b['complete'] !== in_array($complete, ['1', 'true', 'yes'], true)) return false;
            return true;
        }));
        $field = $query->sortField;
        usort($rows, fn(array $a, array $b) => $query->descending ? ($b[$field] <=> $a[$field]) : ($a[$field] <=> $b[$field]));
        return Json::response($response, 200, ['status' => 'success'] + $query->result(array_slice($rows, $query->offset(), $query->pageSize), count($rows)));
    }

    public function create(Request $request, Response $response): Response
    {
        $body = JsonBody::of($request);
        $editionId = is_string($body['edition_id'] ?? null) ? $body['edition_id'] : '';
        $count = isset($body['count']) ? (int) $body['count'] : null;
        $start = isset($body['start_number']) ? (int) $body['start_number'] : 1;
        $manifest = $this->batches->create($editionId, $count, $start, ($body['force'] ?? false) === true);
        return Json::response($response, 201, ['status' => 'success', 'batch' => $manifest]);
    }

    /** @param array{filename: string} $args */
    public function get(Request $request, Response $response, array $args): Response
    {
        return Json::response($response, 200, ['status' => 'success', 'batch' => $this->batches->get($args['filename'])]);
    }

    /** @param array{filename: string} $args */
    public function save(Request $request, Response $response, array $args): Response
    {
        $body = JsonBody::of($request);
        $manifest = $body['manifest'] ?? $body;
        if (!is_array($manifest)) throw new ApiError(422, 'invalid_manifest', 'A manifest is a JSON object.');
        $this->batches->get($args['filename']); // 404 for unknown batches (create them with POST)
        $this->batches->save($args['filename'], $manifest);
        return Json::response($response, 200, ['status' => 'success', 'batch' => $this->batches->get($args['filename'])]);
    }

    /** @param array{filename: string} $args */
    public function zip(Request $request, Response $response, array $args): Response
    {
        $path = $this->batches->zip($args['filename']);
        return Download::file($response, $path, 'application/zip', basename($path));
    }

    /** @param array{type: string, filename: string} $args */
    public function putFile(Request $request, Response $response, array $args): Response
    {
        $body = JsonBody::of($request);
        $doc = is_string($body['doc'] ?? null) ? $body['doc'] : '';
        $this->batches->writeFile($args['type'], $args['filename'], $doc);
        return Json::response($response, 200, ['status' => 'success', 'type' => $args['type'], 'filename' => $args['filename']]);
    }

    /** @param array{type: string, filename: string} $args */
    public function getFile(Request $request, Response $response, array $args): Response
    {
        if (!$this->files->exists($args['type'], $args['filename'])) throw new ApiError(404, 'not_found', 'File not found.');
        $type = match (pathinfo($args['filename'], PATHINFO_EXTENSION)) {
            'pdf' => 'application/pdf',
            'svg' => 'image/svg+xml',
            'zip' => 'application/zip',
            default => 'application/json',
        };
        return Download::file($response, $this->files->path($args['type'], $args['filename']), $type, $args['filename']);
    }
}
