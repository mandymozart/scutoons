<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\Accounts\AccountService;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;

/**
 *   GET /api/me/   { user, access, memberships, inventory, avatar }
 *   PUT /api/me/   { marketing_opt_in?, hide_sensitive?, media_quality?, display_name? } → { status, user, avatar }
 */
final class MeController
{
    public function __construct(private readonly AccountService $accounts)
    {
    }

    public function get(Request $request, Response $response): Response
    {
        return Json::response($response, 200, $this->accounts->me(CurrentSession::of($request)->requireUser()));
    }

    public function put(Request $request, Response $response): Response
    {
        $body = JsonBody::of($request);
        $user = CurrentSession::of($request)->requireUser();
        return Json::response($response, 200, ['status' => 'success'] + $this->accounts->update($user, $body));
    }
}
