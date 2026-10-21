<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers\Admin;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\Admin\ContentAdmin;
use Scutoons\Domain\Admin\VersionPurge;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;

/**
 * /api/admin – role admin (docs/API.md "Admin API"):
 *   GET /content   the content store overview (?toon=): every stored file with its newest / published version
 *   GET /content/old-versions?before=YYYY-MM-DD   the dry run of the purge (issue 154, VersionPurge)
 *   POST /content/old-versions/purge { before, documents, versions }   deletes them when the dry run's counts still hold
 */
final class ContentController
{
    public function __construct(private readonly ContentAdmin $content, private readonly VersionPurge $purge)
    {
    }

    public function index(Request $request, Response $response): Response
    {
        return Json::response($response, 200, ['status' => 'success'] + $this->content->index($request->getQueryParams()['toon'] ?? null));
    }

    public function oldVersions(Request $request, Response $response): Response
    {
        return Json::response($response, 200, ['status' => 'success'] + $this->purge->plan($request->getQueryParams()['before'] ?? null));
    }

    public function purgeOldVersions(Request $request, Response $response): Response
    {
        $body = JsonBody::of($request);
        return Json::response($response, 200, ['status' => 'success'] + $this->purge->purge($body['before'] ?? null, $body['documents'] ?? null, $body['versions'] ?? null));
    }
}
