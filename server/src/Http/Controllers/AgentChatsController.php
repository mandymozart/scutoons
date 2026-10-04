<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\Agent\AgentChats;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;
use Scutoons\Support\Json as JsonEncoding;

/**
 * The creator's Ask panel chat history (docs/API.md "Agent chats"; a content editor, like /api/entities – each only
 * ever sees and changes their own chats):
 *   GET    /api/agent-chats?limit=            my chats, newest updated first (no content; `turns` = count)
 *   GET    /api/agent-chats/<id>[?messages=1] one chat with its turns (+ the model messages)
 *   PUT    /api/agent-chats/<id>              { title?, page?, turns, messages } → create or replace
 *   DELETE /api/agent-chats/<id>
 */
final class AgentChatsController
{
    public function __construct(private readonly AgentChats $chats, private readonly Roles $roles)
    {
    }

    public function index(Request $request, Response $response): Response
    {
        $user = $this->creator($request);
        $limit = (int) ($request->getQueryParams()['limit'] ?? 50);
        return Json::response($response, 200, ['status' => 'success', 'chats' => $this->chats->list((string) $user['id'], $limit)]);
    }

    /** @param array{id: string} $args */
    public function get(Request $request, Response $response, array $args): Response
    {
        $user = $this->creator($request);
        $messages = ($request->getQueryParams()['messages'] ?? '') === '1';
        $chat = $this->chats->get((string) $user['id'], $args['id'], $messages);
        return Json::response($response, 200, ['status' => 'success', 'chat' => $chat], [], JsonEncoding::FLAGS | JSON_PRESERVE_ZERO_FRACTION);
    }

    /** @param array{id: string} $args */
    public function put(Request $request, Response $response, array $args): Response
    {
        $user = $this->creator($request);
        if (strlen(JsonBody::raw($request)) > AgentChats::MAX_BYTES) {
            throw new ApiError(413, 'too_large', 'A chat is limited to ' . AgentChats::MAX_BYTES . ' bytes.');
        }
        $body = JsonBody::object($request) ?? throw new ApiError(422, 'invalid_body', 'The request body must be a JSON object { turns, messages }.');
        return Json::response($response, 200, ['status' => 'success', 'chat' => $this->chats->save((string) $user['id'], $args['id'], $body)]);
    }

    /** @param array{id: string} $args */
    public function delete(Request $request, Response $response, array $args): Response
    {
        $user = $this->creator($request);
        $this->chats->delete((string) $user['id'], $args['id']);
        return Json::response($response, 200, ['status' => 'success']);
    }

    /** @return array<string, mixed> */
    private function creator(Request $request): array
    {
        $session = CurrentSession::of($request);
        $session->rejectCsrf();
        return $this->roles->requireContentEditor($session->user());
    }
}
