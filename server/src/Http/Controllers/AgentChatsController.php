<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\Agent\AgentApps;
use Scutoons\Domain\Agent\AgentChats;
use Scutoons\Domain\ApiError;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;
use Scutoons\Support\Json as JsonEncoding;

/**
 * The Ask panel chat history of the creator, the Embassy and the admin app (docs/API.md "Agent chats"; who may use an
 * app's panel: AgentApps – each user only ever sees and changes their own chats of that app):
 *   GET    /api/agent-chats?app=&limit=            my chats of the app (default creator), newest updated first
 *                                                  (no content; `turns` = count)
 *   GET    /api/agent-chats/<id>[?messages=1&app=] one chat with its turns (+ the model messages)
 *   PUT    /api/agent-chats/<id>                   { app?, title?, page?, turns, messages } → create or replace
 *   DELETE /api/agent-chats/<id>[?app=]
 * By id, the role check follows the stored chat's app (`?app=` given and different → 404).
 */
final class AgentChatsController
{
    public function __construct(private readonly AgentChats $chats, private readonly AgentApps $apps)
    {
    }

    public function index(Request $request, Response $response): Response
    {
        $query = $request->getQueryParams();
        $app = AgentApps::app($query['app'] ?? null);
        $user = $this->apps->requireUser($app, $this->session($request));
        $limit = (int) ($query['limit'] ?? 50);
        return Json::response($response, 200, ['status' => 'success', 'chats' => $this->chats->list((string) $user['id'], $limit, $app)]);
    }

    /** @param array{id: string} $args */
    public function get(Request $request, Response $response, array $args): Response
    {
        [$user, $app] = $this->byId($request, $args['id']);
        $messages = ($request->getQueryParams()['messages'] ?? '') === '1';
        $chat = $this->chats->get((string) $user['id'], $args['id'], $messages, $app);
        return Json::response($response, 200, ['status' => 'success', 'chat' => $chat], [], JsonEncoding::FLAGS | JSON_PRESERVE_ZERO_FRACTION);
    }

    /** @param array{id: string} $args */
    public function put(Request $request, Response $response, array $args): Response
    {
        $session = $this->session($request) ?? throw new ApiError(401, 'unauthorized', 'Not signed in.');
        if (strlen(JsonBody::raw($request)) > AgentChats::MAX_BYTES) {
            throw new ApiError(413, 'too_large', 'A chat is limited to ' . AgentChats::MAX_BYTES . ' bytes.');
        }
        $body = JsonBody::object($request) ?? throw new ApiError(422, 'invalid_body', 'The request body must be a JSON object { turns, messages }.');
        $app = AgentApps::app($body->app ?? null);
        $user = $this->apps->requireUser($app, $session);
        return Json::response($response, 200, ['status' => 'success', 'chat' => $this->chats->save((string) $user['id'], $args['id'], $body, $app)]);
    }

    /** @param array{id: string} $args */
    public function delete(Request $request, Response $response, array $args): Response
    {
        [$user, $app] = $this->byId($request, $args['id']);
        $this->chats->get((string) $user['id'], $args['id'], false, $app); // 404 for another app's chat
        $this->chats->delete((string) $user['id'], $args['id']);
        return Json::response($response, 200, ['status' => 'success']);
    }

    /**
     * The user for a request by chat id: role check by the stored chat's app (else by `?app=`, default creator)
     * @return array{0: array<string, mixed>, 1: string} the user and the app the chat must belong to
     */
    private function byId(Request $request, string $id): array
    {
        $user = $this->session($request) ?? throw new ApiError(401, 'unauthorized', 'Not signed in.');
        $asked = $request->getQueryParams()['app'] ?? null;
        $asked = $asked !== null && $asked !== '' ? AgentApps::app($asked) : null;
        $app = $this->chats->appOf((string) $user['id'], $id) ?? $asked ?? AgentApps::CREATOR;
        return [$this->apps->requireUser($app, $user), $asked ?? $app];
    }

    /** @return array<string, mixed>|null the signed-in user (cookie requests from a foreign origin are refused) */
    private function session(Request $request): ?array
    {
        $session = CurrentSession::of($request);
        $session->rejectCsrf();
        return $session->user();
    }
}
