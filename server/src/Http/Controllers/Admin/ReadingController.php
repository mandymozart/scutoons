<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers\Admin;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Progress\ProgressService;
use Scutoons\Domain\Progress\ReadingReport;
use Scutoons\Http\Json;

/**
 * /api/admin/reading – role admin (BACKLOG 10.69):
 *   GET /reading?toon=   per chapter: members who started / finished it and where the unfinished ones stopped
 *                        → { readers, chapters: [{ toon, chapter, started, finished, reading, stops[10], last_read_at }] }
 */
final class ReadingController
{
    public function __construct(private readonly ReadingReport $reading)
    {
    }

    public function stats(Request $request, Response $response): Response
    {
        $toon = $request->getQueryParams()['toon'] ?? null;
        if ($toon === '') $toon = null;
        if ($toon !== null && (!is_string($toon) || !preg_match('~^' . ProgressService::TOON_PATTERN . '$~', $toon))) {
            throw new ApiError(400, 'invalid_toon', 'toon must be a toon id.');
        }
        return Json::response($response, 200, ['status' => 'success'] + $this->reading->chapterStats($toon));
    }
}
