<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers\Admin;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\Admin\ContentAdmin;
use Scutoons\Http\Json;

/**
 * /api/admin – role admin (docs/API.md "Admin API"):
 *   GET /content   the content store overview (?toon=): every stored file with its newest / published version
 */
final class ContentController
{
    public function __construct(private readonly ContentAdmin $content)
    {
    }

    public function index(Request $request, Response $response): Response
    {
        return Json::response($response, 200, ['status' => 'success'] + $this->content->index($request->getQueryParams()['toon'] ?? null));
    }
}
