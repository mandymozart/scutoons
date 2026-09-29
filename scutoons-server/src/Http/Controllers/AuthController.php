<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\Auth\LoginService;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;

/**
 *   POST /api/auth/request/  { email, code?, item_id?, variant?, locale?, marketing_opt_in? } → { status: "sent", expires_at }
 *   POST /api/auth/verify/   { email, otp } | { token } → { status, session_token, user, created, claimed?, claim_error? }
 *   POST /api/auth/logout/   → { status: "success" } (this session only)
 */
final class AuthController
{
    public function __construct(private readonly LoginService $login, private readonly Sessions $sessions)
    {
    }

    public function request(Request $request, Response $response): Response
    {
        return Json::response($response, 200, $this->login->request(JsonBody::of($request)));
    }

    public function verify(Request $request, Response $response): Response
    {
        return Json::response($response, 200, $this->login->verify(JsonBody::of($request)));
    }

    public function logout(Request $request, Response $response): Response
    {
        $token = CurrentSession::of($request)->token();
        if ($token !== null) $this->sessions->end($token);
        return Json::response($response, 200, ['status' => 'success']);
    }
}
