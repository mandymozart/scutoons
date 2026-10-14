<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers\Admin;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\Admin\Segments;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;

/**
 * /api/admin/segments – role admin (agents/research/GIFT-INVITES.md 1b; Domain\Admin\Segments):
 *   GET    /segments                         { items: [{ id, name, members, created_at }] }
 *   POST   /segments                         { name } → { item }
 *   PUT    /segments/{id}                    { name } → { item }
 *   DELETE /segments/{id}                    (409 in_use: a campaign lets it invite)
 *   POST   /segments/{id}/members            { ids } | { matching: true, q?, filters? } → { added, segment }
 *   POST   /segments/{id}/members/remove     { ids } → { removed, segment }
 * The Users list filters by segment (GET /admin/users?segment=…) and lists each account's segments.
 */
final class SegmentsController
{
    public function __construct(private readonly Segments $segments)
    {
    }

    public function list(Request $request, Response $response): Response
    {
        return Json::response($response, 200, ['status' => 'success', 'items' => $this->segments->list()]);
    }

    public function create(Request $request, Response $response): Response
    {
        $admin = CurrentSession::of($request)->requireUser();
        return Json::response($response, 201, ['status' => 'success', 'item' => $this->segments->create($admin, JsonBody::of($request)['name'] ?? null)]);
    }

    /** @param array{id: string} $args */
    public function rename(Request $request, Response $response, array $args): Response
    {
        return Json::response($response, 200, ['status' => 'success', 'item' => $this->segments->rename($args['id'], JsonBody::of($request)['name'] ?? null)]);
    }

    /** @param array{id: string} $args */
    public function delete(Request $request, Response $response, array $args): Response
    {
        $this->segments->delete($args['id']);
        return Json::response($response, 200, ['status' => 'success']);
    }

    /** @param array{id: string} $args */
    public function add(Request $request, Response $response, array $args): Response
    {
        $admin = CurrentSession::of($request)->requireUser();
        return Json::response($response, 200, ['status' => 'success'] + $this->segments->add($admin, $args['id'], JsonBody::of($request)));
    }

    /** @param array{id: string} $args */
    public function remove(Request $request, Response $response, array $args): Response
    {
        return Json::response($response, 200, ['status' => 'success'] + $this->segments->remove($args['id'], JsonBody::of($request)));
    }
}
