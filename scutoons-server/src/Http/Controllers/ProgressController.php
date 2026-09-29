<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\Progress\ProgressService;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;

/**
 *   GET /api/progress/<toonId>/  → { toon_id, state, updated_at }
 *   PUT /api/progress/<toonId>/  { state, updated_at } → { status: "success"|"stale", toon_id, state, updated_at }
 */
final class ProgressController
{
    public function __construct(private readonly ProgressService $progress)
    {
    }

    /** @param array{toon: string} $args */
    public function get(Request $request, Response $response, array $args): Response
    {
        $user = CurrentSession::of($request)->requireUser();
        return Json::response($response, 200, $this->progress->get((string) $user['id'], $args['toon']));
    }

    /** @param array{toon: string} $args */
    public function put(Request $request, Response $response, array $args): Response
    {
        $body = JsonBody::of($request);
        $user = CurrentSession::of($request)->requireUser();
        // The state is re-decoded with objects so an empty {} stays an object
        $state = JsonBody::object($request)->state ?? null;
        return Json::response($response, 200, $this->progress->put((string) $user['id'], $args['toon'], $state, $body['updated_at'] ?? null));
    }
}
