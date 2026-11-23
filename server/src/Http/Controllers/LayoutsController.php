<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\Layouts\WorkspaceLayouts;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;

/**
 * A member's saved creator workspace layouts (Domain\Layouts) – signed in, own rows only; an AI connection only reads:
 *   GET    /api/me/layouts              { available, layouts: [{ workspace, kind, name, layout, saved_at }], defaults: [{ workspace, craft, layout }] }
 *   PUT    /api/me/layouts/{workspace}  { layout, name?, kind? } → { layout: row }
 *   DELETE /api/me/layouts/{workspace}  → { deleted }: back to the default (an own workspace: gone)
 */
final class LayoutsController
{
    public function __construct(private readonly WorkspaceLayouts $layouts)
    {
    }

    public function mine(Request $request, Response $response): Response
    {
        $user = CurrentSession::of($request)->requireUser();
        return Json::response($response, 200, ['status' => 'success'] + $this->layouts->of((string) $user['id']));
    }

    /** @param array<string, string> $args */
    public function save(Request $request, Response $response, array $args): Response
    {
        $body = JsonBody::of($request);
        $user = CurrentSession::of($request)->requireUser();
        return Json::response($response, 200, ['status' => 'success', 'layout' => $this->layouts->save((string) $user['id'], $args['workspace'] ?? '', $body)]);
    }

    /** @param array<string, string> $args */
    public function delete(Request $request, Response $response, array $args): Response
    {
        $user = CurrentSession::of($request)->requireUser();
        return Json::response($response, 200, ['status' => 'success', 'deleted' => $this->layouts->delete((string) $user['id'], $args['workspace'] ?? '')]);
    }
}
