<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\Avatar\AvatarService;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;

/**
 *   GET /api/avatar/  → { status, avatar: { slots, body, unlocked } }
 *   PUT /api/avatar/  { slots?: { <slot>: entryId|null }, body?: { character, look }|null } → { status, avatar, restored }
 *                     (restored = slot instances the cover rule refilled, agents/ITEMS.md "Wardrobe")
 */
final class AvatarController
{
    public function __construct(private readonly AvatarService $avatar)
    {
    }

    public function get(Request $request, Response $response): Response
    {
        $user = CurrentSession::of($request)->requireUser();
        $this->avatar->ensureWardrobe((string) $user['id']);
        return Json::response($response, 200, ['status' => 'success', 'avatar' => $this->avatar->toJson((string) $user['id'])]);
    }

    public function put(Request $request, Response $response): Response
    {
        $body = JsonBody::of($request);
        $user = CurrentSession::of($request)->requireUser();
        $this->avatar->ensureWardrobe((string) $user['id']);
        $restored = $this->avatar->update((string) $user['id'], $body);
        return Json::response($response, 200, ['status' => 'success', 'avatar' => $this->avatar->toJson((string) $user['id']), 'restored' => $restored]);
    }
}
