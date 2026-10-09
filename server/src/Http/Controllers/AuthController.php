<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\Accounts\Users;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Auth\LoginService;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Http\AllowedOrigins;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;
use Scutoons\Http\SessionCookie;
use Scutoons\Infrastructure\Config\Settings;

/**
 *   POST /api/auth/request/  { email, code?, item_id?, variant?, locale?, marketing_opt_in? } → { status: "sent", expires_at }
 *   POST /api/auth/verify/   { email, otp } | { token } → { status, session_token, user, created, claimed?, claim_error? }
 *                            + the single sign-on cookie when the request comes from an ALLOWED_ORIGINS app
 *   POST /api/auth/logout/   { all?: true } → { status: "success" } – ends this session (all: every session of the
 *                            account, "sign out on all devices") and clears the cookie
 *   GET  /api/auth/session/  → { status, user, via: "cookie"|"bearer" } ({ user: null, via: null } when signed out);
 *                            renews the cookie
 *   POST /api/auth/session/  Bearer → the same session as the cookie (one-time move of an app's stored token) → same answer
 * Single sign-on: docs/API.md "Single sign-on".
 */
final class AuthController
{
    public function __construct(
        private readonly LoginService $login,
        private readonly Sessions $sessions,
        private readonly Users $users,
        private readonly SessionCookie $cookie,
        private readonly Settings $settings,
    ) {
    }

    public function request(Request $request, Response $response): Response
    {
        return Json::response($response, 200, $this->login->request(JsonBody::of($request)));
    }

    public function verify(Request $request, Response $response): Response
    {
        $result = $this->login->verify(JsonBody::of($request));
        $response = Json::response($response, 200, $result);
        // Only a SCUTOONS app gets the cookie (a form on another site must not sign the browser into its account)
        if ($this->fromApp($request) && is_string($result['session_token'] ?? null)) {
            $response = $this->cookie->set($request, $response, $result['session_token']);
        }
        return $response;
    }

    public function logout(Request $request, Response $response): Response
    {
        $session = CurrentSession::of($request);
        $session->rejectCsrf();
        $token = $session->token();
        $all = ($this->body($request)['all'] ?? false) === true;
        if ($token !== null) {
            $user = $all ? $session->user() : null;
            if ($user !== null) $this->sessions->endAll((string) $user['id']);
            else $this->sessions->end($token);
        }
        $response = Json::response($response, 200, ['status' => 'success']);
        $cookieToken = SessionCookie::token($request);
        if ($cookieToken !== null && ($session->via() !== CurrentSession::VIA_BEARER || $cookieToken === $token || $all)) {
            $response = $this->cookie->clear($request, $response);
        }
        return $response;
    }

    /**
     * GET /auth/session/: who is signed in (cookie or Bearer) – `user: null` when nobody is (200, so a guest's check is
     * not an error in the browser console); the cookie's lifetime starts again
     */
    public function session(Request $request, Response $response): Response
    {
        $session = CurrentSession::of($request);
        $user = $session->user();
        if ($user === null) return Json::response($response, 200, ['status' => 'success', 'user' => null, 'via' => null]);
        $response = $this->answer($response, $user, (string) $session->via());
        $token = $session->token();
        if ($session->via() === CurrentSession::VIA_COOKIE && $token !== null) {
            $response = $this->cookie->set($request, $response, $token);
        }
        return $response;
    }

    /** POST /auth/session/: an app's stored Bearer token becomes the cookie (the app then drops its copy) */
    public function exchange(Request $request, Response $response): Response
    {
        $session = CurrentSession::of($request);
        if ($session->via() !== CurrentSession::VIA_BEARER || $session->isApiToken()) {
            throw new ApiError(400, 'bearer_required', 'Send the stored session token as Bearer.');
        }
        if (!$this->fromApp($request)) {
            throw new ApiError(403, 'csrf_rejected', 'Only the SCUTOONS apps can move a session into the cookie.');
        }
        $user = $session->requireUser();
        return $this->cookie->set($request, $this->answer($response, $user, CurrentSession::VIA_COOKIE), (string) $session->token());
    }

    /** @param array<string, mixed> $user */
    private function answer(Response $response, array $user, string $via): Response
    {
        return Json::response($response, 200, ['status' => 'success', 'user' => $this->users->toJson($user), 'via' => $via]);
    }

    private function fromApp(Request $request): bool
    {
        return AllowedOrigins::allows($this->settings, $request->getHeaderLine('Origin'));
    }

    /** @return array<mixed> the body, [] when there is none or it is not JSON (logout never failed on its body) */
    private function body(Request $request): array
    {
        try {
            return JsonBody::of($request);
        } catch (ApiError) {
            return [];
        }
    }
}
