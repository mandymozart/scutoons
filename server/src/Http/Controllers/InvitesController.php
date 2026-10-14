<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Campaigns\Invites;
use Scutoons\Domain\Inventory\InventoryService;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;
use Scutoons\Infrastructure\Config\Settings;

/**
 * Gift invites (agents/research/GIFT-INVITES.md; Domain\Campaigns\Invites) – signed in (Bearer):
 *   GET  /api/invites                 { campaign: offer | null, invites, address_needed }
 *   POST /api/invites                 { to, entry_id | item_id (+ variant), qty? } → { invite, inventory }
 *   POST /api/invites/{id}/recall     → { invite, inventory } (only while not accepted)
 *   PUT  /api/invites/{id}/address    { name, address, country } → mine (a physical gift the friend joined by)
 * Cron (no session): GET|POST /api/invites/cron?key=CAMPAIGN_CRON_KEY → { result: { returned, unpaid } } – returns
 * the invites that ran out (also done on every GET /api/invites and in the admin; the cron only makes it timely).
 */
final class InvitesController
{
    public function __construct(
        private readonly Invites $invites,
        private readonly InventoryService $inventory,
        private readonly Settings $settings,
    ) {
    }

    public function mine(Request $request, Response $response): Response
    {
        $user = CurrentSession::of($request)->requireUser();
        return Json::response($response, 200, ['status' => 'success'] + $this->invites->mine($user));
    }

    public function create(Request $request, Response $response): Response
    {
        $body = JsonBody::of($request);
        $user = CurrentSession::of($request)->requireUser();
        $invite = $this->invites->create($user, $body);
        return Json::response($response, 200, ['status' => 'success', 'invite' => $invite, 'inventory' => $this->inventory->entries((string) $user['id'])]);
    }

    /** @param array{id: string} $args */
    public function recall(Request $request, Response $response, array $args): Response
    {
        $user = CurrentSession::of($request)->requireUser();
        $invite = $this->invites->recall($user, $args['id']);
        return Json::response($response, 200, ['status' => 'success', 'invite' => $invite, 'inventory' => $this->inventory->entries((string) $user['id'])]);
    }

    /** @param array{id: string} $args */
    public function address(Request $request, Response $response, array $args): Response
    {
        $body = JsonBody::of($request);
        $user = CurrentSession::of($request)->requireUser();
        return Json::response($response, 200, ['status' => 'success'] + $this->invites->setAddress($user, $args['id'], $body));
    }

    public function cron(Request $request, Response $response): Response
    {
        $key = $this->settings->get('CAMPAIGN_CRON_KEY');
        if ($key === '') throw new ApiError(404, 'not_found', 'Not found.');
        $given = $request->getQueryParams()['key'] ?? null;
        $given = is_string($given) && $given !== '' ? $given : (string) ($request->getServerParams()['PHP_AUTH_PW'] ?? '');
        if ($given === '' || !hash_equals($key, $given)) throw new ApiError(403, 'forbidden', 'Wrong key.');
        return Json::response($response, 200, ['status' => 'success', 'result' => $this->invites->sweep()]);
    }
}
