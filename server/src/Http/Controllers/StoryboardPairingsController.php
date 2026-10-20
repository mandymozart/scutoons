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
 * The handheld camera's pairing (#126, docs/API.md "Storyboard" → "Handheld pairing"; Domain\Storyboard\Pairings):
 *   POST   /api/storyboard/pairings          { toon, chapter, shot? } (content editor) → 201 { status, pairing }
 *   PUT    /api/storyboard/pairings/{code}/offer   { sdp }  no session – the code is the right → { status }
 *   PUT    /api/storyboard/pairings/{code}/answer  { sdp }  no session → { status }
 *   GET    /api/storyboard/pairings/{code}   no session → { status, pairing } (its creator also gets `answer`; that read
 *                                            ends the pairing)
 *   DELETE /api/storyboard/pairings/{code}   (its creator) → { status }
 * The routes without a session are rate-limited per client IP (config/routes.php).
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
    public function offer(Request $request, Response $response, array $args): Response
    {
        $this->pairings->offer($args['code'], JsonBody::of($request));
        return Json::response($response, 200, ['status' => 'success']);
    }

    /** @param array{code: string} $args */
    public function answer(Request $request, Response $response, array $args): Response
    {
        $this->pairings->answer($args['code'], JsonBody::of($request));
        return Json::response($response, 200, ['status' => 'success']);
    }

    /** @param array{code: string} $args */
    public function get(Request $request, Response $response, array $args): Response
    {
        $pairing = $this->pairings->read($args['code'], CurrentSession::of($request)->user());
        return Json::response($response, 200, ['status' => 'success', 'pairing' => $pairing])->withHeader('Cache-Control', 'no-store');
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
