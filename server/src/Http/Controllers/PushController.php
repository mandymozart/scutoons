<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\Push\PushSender;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Push\PushSubscriptions;
use Scutoons\Domain\Push\TeamPush;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;

/**
 * Web Push opt-in (agents/research/PWA.md §2b – the reader's account page; research/PWA-APPS.md §5 – the tools'
 * "Benachrichtigungen"):
 *   GET  /api/push/key          { key: <VAPID public key, base64url> | null }  (null = push is off on this server)
 *   POST /api/push/subscribe    { subscription: { endpoint, keys: { p256dh, auth } }, locale, app? } → { status: success }
 *   POST /api/push/unsubscribe  { endpoint } → { status: success }
 *   GET  /api/push/prefs?app=   signed in: { key, kinds: [{ kind, on, push }] } – this account's kinds in a tool
 *   PUT  /api/push/prefs        signed in: { kind, on } → the same answer for the kind's app
 *   GET|POST /api/push/cron?key=PUSH_CRON_KEY  the host's cron (every 5 min): the drafts kind (TeamPush::cron)
 * Subscribing needs no account (a signed-in account is remembered with the browser); a per-IP limit (config/routes.php).
 */
final class PushController
{
    public function __construct(
        private readonly PushSubscriptions $subscriptions,
        private readonly PushSender $sender,
        private readonly TeamPush $team,
        private readonly Settings $settings,
    )
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

    public function prefs(Request $request, Response $response): Response
    {
        $user = CurrentSession::of($request)->requireUser();
        return Json::response($response, 200, ['status' => 'success'] + $this->team->prefs($user, TeamPush::app($request->getQueryParams()['app'] ?? null)));
    }

    public function setPref(Request $request, Response $response): Response
    {
        $user = CurrentSession::of($request)->requireUser();
        $body = JsonBody::of($request);
        $this->team->setPref($user, $body['kind'] ?? null, $body['on'] ?? null);
        return Json::response($response, 200, ['status' => 'success'] + $this->team->prefs($user, TeamPush::KINDS[(string) $body['kind']]['app']));
    }

    /** the host's cron: GET|POST /api/push/cron?key=… (or the HTTP Basic password); no key configured = 404 */
    public function cron(Request $request, Response $response): Response
    {
        $key = $this->settings->get('PUSH_CRON_KEY');
        if ($key === '') throw new ApiError(404, 'not_found', 'Not found.');
        $given = $request->getQueryParams()['key'] ?? null;
        $given = is_string($given) && $given !== '' ? $given : (string) ($request->getServerParams()['PHP_AUTH_PW'] ?? '');
        if ($given === '' || !hash_equals($key, $given)) throw new ApiError(403, 'forbidden', 'Wrong key.');
        return Json::response($response, 200, ['status' => 'success', 'result' => $this->team->cron()]);
    }
}
