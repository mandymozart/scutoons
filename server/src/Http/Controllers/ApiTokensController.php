<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\Auth\ApiTokens;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;

/**
 * /api/me/tokens – the signed-in member's AI connections (personal access tokens, Domain\Auth\ApiTokens, agents/MCP.md
 * §3); the account page's "AI connections". A real session only (an AI connection can't manage tokens – ApiTokenScope).
 *   GET    /me/tokens        → { status, member, items: [{ id, name, created_at, last_used_at, expires_at }] }
 *                              (member: the account may have tokens – readers can't)
 *   POST   /me/tokens        { name } → { status, token, item } (201; `token` is shown this once)
 *                              · 403 no_member_role · 400 name_required · 409 too_many
 *   DELETE /me/tokens/{id}   → { status } (revoked: the next call with it answers 401)
 */
final class ApiTokensController
{
    public function __construct(private readonly ApiTokens $tokens)
    {
    }

    public function list(Request $request, Response $response): Response
    {
        $user = CurrentSession::of($request)->requireUser();
        $member = $this->tokens->isMember($user);
        return Json::response($response, 200, ['status' => 'success', 'member' => $member, 'items' => $member ? $this->tokens->list($user) : []]);
    }

    public function create(Request $request, Response $response): Response
    {
        $user = CurrentSession::of($request)->requireUser();
        $result = $this->tokens->create($user, (string) (JsonBody::of($request)['name'] ?? ''));
        return Json::response($response, 201, ['status' => 'success'] + $result);
    }

    /** @param array{id: string} $args */
    public function revoke(Request $request, Response $response, array $args): Response
    {
        $this->tokens->revoke(CurrentSession::of($request)->requireUser(), $args['id']);
        return Json::response($response, 200, ['status' => 'success']);
    }
}
