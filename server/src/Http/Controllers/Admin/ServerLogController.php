<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers\Admin;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\Admin\ServerLog;
use Scutoons\Domain\ApiError;
use Scutoons\Http\Json;
use Scutoons\Infrastructure\Config\Settings;

/**
 * /api/admin/server-errors – role admin (docs/API.md "Admin API" → Server errors, BACKLOG 10.11): the newest entries of
 * a PHP error log, newest first, messages only (ServerLog masks e-mail addresses, tokens, passwords).
 *   GET /server-errors?source=api|qr&day=YYYY-MM-DD&q=&limit=200
 *       { status, source, sources: [{ id, configured }], exists, size, truncated, days, entries: [{ at, day, level, message, detail }] }
 * `source`: api = LOG_PATH (this API), qr = QR_LOG_PATH (the qr app). The log is read from its last 512 KB.
 */
final class ServerLogController
{
    private const SOURCES = ['api' => 'LOG_PATH', 'qr' => 'QR_LOG_PATH'];

    public function __construct(private readonly Settings $settings)
    {
    }

    public function list(Request $request, Response $response): Response
    {
        $params = $request->getQueryParams();
        $source = (string) ($params['source'] ?? 'api');
        if (!isset(self::SOURCES[$source])) throw new ApiError(400, 'invalid_source', 'source is api or qr.');
        $day = (string) ($params['day'] ?? '');
        if ($day !== '' && preg_match('/^\d{4}-\d{2}-\d{2}$/', $day) !== 1) throw new ApiError(400, 'invalid_day', 'day is YYYY-MM-DD.');
        $limit = (int) ($params['limit'] ?? 200);
        $root = dirname($this->settings->root());
        $log = ServerLog::read($this->settings->path(self::SOURCES[$source]), $day, (string) ($params['q'] ?? ''), $limit, str_replace('\\', '/', $root));
        $sources = [];
        foreach (self::SOURCES as $id => $key) $sources[] = ['id' => $id, 'configured' => $this->settings->path($key) !== ''];
        return Json::response($response, 200, ['status' => 'success', 'source' => $source, 'sources' => $sources] + $log);
    }
}
