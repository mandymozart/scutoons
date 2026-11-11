<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Live\LiveSessions;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Json;

/**
 * A client's proof for a live session (#478, owner 2026-11-10 [role]; docs/API.md "Live"): the facts a role's filter asks
 * that only this API can state, signed for the relay (LiveSessions::proof):
 *   GET /api/live/proof?session=<code> → { status, proof, expires_in, dev }
 * A signed-in member (cookie or session Bearer): its account roles among member, creator, admin, live. An approved deck's
 * own token (`scd_…`): `approved`, and the account roles of the member it is paired to (#481, owner 2026-11-11
 * [deck-access]: a paired deck is tied to an account, its rights are that account's; before: no account fact). An AI
 * connection 403 not_for_ai, nobody 401, a code that is no session code 400 invalid_session.
 */
final class LiveProofController
{
    public function __construct(private readonly LiveSessions $sessions, private readonly Roles $roles)
    {
    }

    public function proof(Request $request, Response $response): Response
    {
        $session = CurrentSession::of($request);
        if ($session->isApiToken()) throw new ApiError(403, 'not_for_ai', 'An AI connection is no deck – a deck asks with its own token or its reader\'s sign-in.');
        $user = $session->requireUser(); // a deck token: approved, not revoked, not expired (Decks::user)
        $code = $request->getQueryParams()['session'] ?? null;
        if (!is_string($code) || preg_match('/^[A-Za-z0-9_-]{1,64}$/', $code) !== 1) {
            throw new ApiError(400, 'invalid_session', 'session: the live session code (letters, digits, - and _).');
        }
        // a deck token's user is the member the deck is paired to (Decks::user)
        $has = $this->roles->of((string) $user['id']);
        $proof = $this->sessions->proof($code, array_values(array_filter(LiveSessions::FACTS, fn(string $r) => in_array($r, $has, true))), $session->isDeckToken());
        return Json::response($response, 200, ['status' => 'success'] + $proof, ['Cache-Control' => 'no-store']);
    }
}
