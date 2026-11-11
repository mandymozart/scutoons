<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Catalog\ProtectedFiles;
use Scutoons\Domain\Decks\Decks;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;

/**
 * Cyberdecks: identity and access (#180, Domain\Decks\Decks, docs/API.md "Decks").
 * The deck (no session – its Ed25519 key is its identity; rate-limited per client IP in config/routes.php):
 *   POST /decks/request   { public_key, build?, class?, hostname? } → { status: pending, id, code, expires_at } (201)
 *                         or { status: approved, id } (200, its approval holds)
 *   POST /decks/challenge { public_key } → 202 { status: pending, code, expires_at } · 200 { status: approved, challenge,
 *                         expires_at } · 401 unknown_deck | request_expired | revoked | expired (→ request again)
 *   POST /decks/token     { public_key, challenge, signature } → { token, expires_at, approval_expires_at, deck }
 *                         · 401 bad_challenge | bad_signature | not_approved | revoked | expired
 * The member (a real sign-in – AI connections and deck tokens are refused, ApiTokenScope / DeckTokenScope):
 *   GET    /me/decks              → { status, items }
 *   GET    /me/decks/pending?code= → { status, deck: { code, hostname, build, class, expires_at } } · 404 unknown_code
 *   POST   /me/decks/approve      { code, days?, idle_days?, name? } → { status, item }
 *   POST   /me/decks/{id}/renew   → { status, item }
 *   DELETE /me/decks/{id}         → { status }
 * A paired deck (its own token only):
 *   GET    /decks/media?path=live/<toon>/<file> → { status, url, expires } – a show medium as a short signed link (#470)
 *                         · 400 invalid_path · 401 · 403 deck_only
 */
final class DecksController
{
    public function __construct(private readonly Decks $decks, private readonly ProtectedFiles $files)
    {
    }

    /**
     * GET /decks/media?path=live/<toon>/<file> – a show medium for a deck with an account (#470, #484; owner 2026-11-10:
     * protected show media only with an account): a paired deck's own token (approved and fresh), or the sign-in of a
     * browser deck in a signed-in viewer – that deck is its user. Never an AI connection (403 deck_only: show media are
     * for the stage, not downloads); nobody = 401.
     */
    public function media(Request $request, Response $response): Response
    {
        $session = CurrentSession::of($request);
        if ($session->token() === null) throw new ApiError(401, 'unauthorized', 'A deck asks with its own token or its user\'s sign-in.');
        if ($session->isApiToken()) throw new ApiError(403, 'deck_only', 'Show media are for a deck: a paired one or a signed-in browser deck.');
        $session->requireUser(); // a deck token: approved, not revoked, not expired (Decks::user); a sign-in: valid
        $path = $request->getQueryParams()['path'] ?? null;
        $signed = $this->files->showMedium(is_string($path) ? $path : '');
        return Json::response($response, 200, ['status' => 'success'] + $signed, ['Cache-Control' => 'no-store']);
    }

    public function request(Request $request, Response $response): Response
    {
        $result = $this->decks->request(JsonBody::of($request));
        return Json::response($response, $result['status'] === 'pending' ? 201 : 200, ['status' => 'success', 'deck' => $result]);
    }

    public function challenge(Request $request, Response $response): Response
    {
        $result = $this->decks->challenge(JsonBody::of($request)['public_key'] ?? null);
        return Json::response($response, $result['status'] === 'pending' ? 202 : 200, ['status' => 'success', 'deck' => $result]);
    }

    public function token(Request $request, Response $response): Response
    {
        return Json::response($response, 200, ['status' => 'success'] + $this->decks->token(JsonBody::of($request)));
    }

    public function list(Request $request, Response $response): Response
    {
        $user = CurrentSession::of($request)->requireUser();
        return Json::response($response, 200, ['status' => 'success', 'items' => $this->decks->list($user)]);
    }

    public function pending(Request $request, Response $response): Response
    {
        $user = CurrentSession::of($request)->requireUser();
        return Json::response($response, 200, ['status' => 'success', 'deck' => $this->decks->pending($user, $request->getQueryParams()['code'] ?? null)]);
    }

    public function approve(Request $request, Response $response): Response
    {
        $user = CurrentSession::of($request)->requireUser();
        return Json::response($response, 200, ['status' => 'success', 'item' => $this->decks->approve($user, JsonBody::of($request))]);
    }

    /** @param array{id: string} $args */
    public function renew(Request $request, Response $response, array $args): Response
    {
        $item = $this->decks->renew(CurrentSession::of($request)->requireUser(), $args['id']);
        return Json::response($response, 200, ['status' => 'success', 'item' => $item]);
    }

    /** @param array{id: string} $args */
    public function revoke(Request $request, Response $response, array $args): Response
    {
        $this->decks->revoke(CurrentSession::of($request)->requireUser(), $args['id']);
        return Json::response($response, 200, ['status' => 'success']);
    }
}
