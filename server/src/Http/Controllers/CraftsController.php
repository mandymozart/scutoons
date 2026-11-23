<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\Crafts\Crafts;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;

/**
 * Crafts (Domain\Crafts – what a member does, not the catalog's crafting) – signed in:
 *   GET /api/crafts        { crafts: [{ id, name, icon, workspaces }] }
 *   GET /api/me/crafts     { crafts, invited, workspaces, ask, can_edit }
 *   PUT /api/me/crafts     { crafts: [id] } → the same; only at the first login, afterwards admins only (403 crafts_locked)
 */
final class CraftsController
{
    public function __construct(private readonly Crafts $crafts)
    {
    }

    public function list(Request $request, Response $response): Response
    {
        CurrentSession::of($request)->requireUser();
        return Json::response($response, 200, ['status' => 'success', 'crafts' => $this->crafts->all()]);
    }

    public function mine(Request $request, Response $response): Response
    {
        $user = CurrentSession::of($request)->requireUser();
        return Json::response($response, 200, ['status' => 'success'] + $this->crafts->state($user));
    }

    public function setMine(Request $request, Response $response): Response
    {
        $body = JsonBody::of($request);
        $user = CurrentSession::of($request)->requireUser();
        return Json::response($response, 200, ['status' => 'success'] + $this->crafts->setOwn($user, $body));
    }
}
