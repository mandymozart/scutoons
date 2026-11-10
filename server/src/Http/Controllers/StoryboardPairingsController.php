<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Domain\Storyboard\Pairings;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;

/**
 * The handheld camera's pairing (#126, #469; docs/API.md "Storyboard" → "Handheld pairing"; Domain\Storyboard\Pairings):
 *   POST   /api/storyboard/pairings              { toon, chapter, shot? } (content editor) → 201 { status, pairing }
 *                                                (with the relay session and the PC's conductor token)
 *   POST   /api/storyboard/pairings/{code}/claim  no session – the code is the right, once → { status, claim }
 *                                                ({ shot, session, token }: the phone's handheld token)
 *   DELETE /api/storyboard/pairings/{code}       (its creator) → { status }
 * The claim is rate-limited per client IP (config/routes.php).
 */
final class StoryboardPairingsController
{
    public function __construct(private readonly Pairings $pairings, private readonly Roles $roles)
    {
    }

    public function create(Request $request, Response $response): Response
    {
        $body = JsonBody::of($request);
        $session = CurrentSession::of($request);
        $user = $this->roles->requireContentEditor($session->user());
        $session->rejectCsrf();
        return Json::response($response, 201, ['status' => 'success', 'pairing' => $this->pairings->create($user, $body)]);
    }

    /** @param array{code: string} $args */
    public function claim(Request $request, Response $response, array $args): Response
    {
        return Json::response($response, 200, ['status' => 'success', 'claim' => $this->pairings->claim($args['code'])])->withHeader('Cache-Control', 'no-store');
    }

    /** @param array{code: string} $args */
    public function delete(Request $request, Response $response, array $args): Response
    {
        $session = CurrentSession::of($request);
        $user = $this->roles->requireContentEditor($session->user());
        $session->rejectCsrf();
        $this->pairings->delete($args['code'], $user);
        return Json::response($response, 200, ['status' => 'success']);
    }
}
