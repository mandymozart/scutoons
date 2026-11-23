<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers\Admin;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\Crafts\Crafts;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;

/**
 * An admin's invites of team members with their crafts (Domain\Crafts) – role admin:
 *   GET    /api/admin/member-invites          { items }
 *   POST   /api/admin/member-invites          { email, crafts } → 201 { invite } (409 has_account)
 *   DELETE /api/admin/member-invites/{id}     only while open
 */
final class MemberInvitesController
{
    public function __construct(private readonly Crafts $crafts)
    {
    }

    public function list(Request $request, Response $response): Response
    {
        return Json::response($response, 200, ['status' => 'success', 'items' => $this->crafts->invites()]);
    }

    public function create(Request $request, Response $response): Response
    {
        $body = JsonBody::of($request);
        $admin = $request->getAttribute('user');
        $by = 'admin:' . (is_array($admin) ? (string) ($admin['email'] ?? '') : '');
        return Json::response($response, 201, ['status' => 'success', 'invite' => $this->crafts->invite($body, $by)]);
    }

    /** @param array{id: string} $args */
    public function delete(Request $request, Response $response, array $args): Response
    {
        $this->crafts->withdraw($args['id']);
        return Json::response($response, 200, ['status' => 'success']);
    }
}
