<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\Homes\Homes;
use Scutoons\Domain\Inventory\InventoryService;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;

/**
 * The avatar's home (agents/research/CAVE.md, Domain\Homes\Homes):
 *   GET    /api/homes/                 → { status, homes: [{ id, place, scene, residents }], max_residents, mover }
 *   POST   /api/homes/place            { entry_id, home_id, x, y, flip? } → { status, inventory } (furniture: stand it / move it)
 *   POST   /api/homes/move-in          { character, home_id, mover? } → { status, homes, inventory } (the WG, the mover)
 *   POST   /api/homes/visits           { home_id } → { status, token, expires_at } (a friend's visit link)
 *   DELETE /api/homes/visits/{token}   → { status }
 *   GET    /api/homes/visit/{token}    (anyone with the link) → { status, home: { place, scene }, furniture, workshop }
 *   POST   /api/map/ride               { network } → { status, network, paid, inventory } (the fare in cucumbers – CAVE.md D8)
 */
final class HomesController
{
    public function __construct(private readonly Homes $homes, private readonly InventoryService $inventory)
    {
    }

    public function list(Request $request, Response $response): Response
    {
        $user = CurrentSession::of($request)->requireUser();
        return Json::response($response, 200, ['status' => 'success', ...$this->homes->forUser((string) $user['id'])]);
    }

    public function place(Request $request, Response $response): Response
    {
        $body = JsonBody::of($request);
        $user = CurrentSession::of($request)->requireUser();
        $this->homes->place((string) $user['id'], $body);
        return Json::response($response, 200, ['status' => 'success', 'inventory' => $this->inventory->entries((string) $user['id'])]);
    }

    public function moveIn(Request $request, Response $response): Response
    {
        $body = JsonBody::of($request);
        $user = CurrentSession::of($request)->requireUser();
        $this->homes->moveIn((string) $user['id'], $body);
        return Json::response($response, 200, ['status' => 'success', ...$this->homes->forUser((string) $user['id']), 'inventory' => $this->inventory->entries((string) $user['id'])]);
    }

    public function createVisit(Request $request, Response $response): Response
    {
        $body = JsonBody::of($request);
        $user = CurrentSession::of($request)->requireUser();
        return Json::response($response, 200, ['status' => 'success', ...$this->homes->createVisit((string) $user['id'], $body['home_id'] ?? null)]);
    }

    /** @param array<string, string> $args */
    public function revokeVisit(Request $request, Response $response, array $args): Response
    {
        $user = CurrentSession::of($request)->requireUser();
        $this->homes->revokeVisit((string) $user['id'], $args['token']);
        return Json::response($response, 200, ['status' => 'success']);
    }

    public function ride(Request $request, Response $response): Response
    {
        $body = JsonBody::of($request);
        $user = CurrentSession::of($request)->requireUser();
        $ride = $this->homes->ride((string) $user['id'], $body['network'] ?? null);
        return Json::response($response, 200, ['status' => 'success', ...$ride, 'inventory' => $this->inventory->entries((string) $user['id'])]);
    }

    /** @param array<string, string> $args */
    public function visit(Request $request, Response $response, array $args): Response
    {
        return Json::response($response, 200, ['status' => 'success', ...$this->homes->visit($args['token'])]);
    }
}
