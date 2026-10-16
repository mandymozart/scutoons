<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\Push\PushSender;
use Scutoons\Domain\Push\PushSubscriptions;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;

/**
 * Web Push opt-in of the reader app (agents/research/PWA.md §2b; the account page's "Notifications"):
 *   GET  /api/push/key          { key: <VAPID public key, base64url> | null }  (null = push is off on this server)
 *   POST /api/push/subscribe    { subscription: { endpoint, keys: { p256dh, auth } }, locale } → { status: success }
 *   POST /api/push/unsubscribe  { endpoint } → { status: success }
 * No account needed (a signed-in account is remembered with the browser); a per-IP limit (config/routes.php).
 */
final class PushController
{
    public function __construct(private readonly PushSubscriptions $subscriptions, private readonly PushSender $sender)
    {
    }

    public function key(Request $request, Response $response): Response
    {
        return Json::response($response, 200, ['status' => 'success', 'key' => $this->sender->configured() ? $this->sender->publicKey() : null]);
    }

    public function subscribe(Request $request, Response $response): Response
    {
        $session = CurrentSession::of($request);
        $this->subscriptions->save(JsonBody::of($request), $session->csrfBlocked() ? null : $session->user());
        return Json::response($response, 200, ['status' => 'success']);
    }

    public function unsubscribe(Request $request, Response $response): Response
    {
        $this->subscriptions->remove(JsonBody::of($request)['endpoint'] ?? null);
        return Json::response($response, 200, ['status' => 'success']);
    }
}
