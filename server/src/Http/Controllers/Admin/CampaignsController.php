<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers\Admin;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\Admin\AccountFilters;
use Scutoons\Domain\Campaigns\CampaignStore;
use Scutoons\Domain\Campaigns\Giveaways;
use Scutoons\Domain\Campaigns\Invites;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;

/**
 * /api/admin/campaigns – role admin (agents/research/GIFT-INVITES.md; Domain\Campaigns):
 *   GET    /campaigns                      { items } (each with counts)
 *   POST   /campaigns                      { name, starts_at, ends_at, gift, item_id?, … } → { item } (a draft)
 *   GET|PUT|DELETE /campaigns/{id}         (DELETE: only without invites – stop it otherwise)
 *   POST   /campaigns/{id}/start|stop      → { item }
 *   GET    /campaigns/{id}/accounts        ?q&page&page_size + the account filters (AccountFilters) → { mode, segment_id, chosen,
 *                                          total, items: [{ id, email, created_at, chosen }] }
 *   PUT    /campaigns/{id}/accounts        { all } | { segment_id } | { add?: [ids], remove?: [ids] } | { add_matching: true, q?,
 *                                          filters? } → accounts (page 1)
 *   GET    /campaigns/{id}/invites         ?status=payment|waiting|joined|returned|recalled|unpaid|to_ship → { items }
 *   GET    /campaigns/{id}/stats           { inviters, sent, by_status, joined_later, members }
 *   POST   /campaigns/invites/{id}/paid    → { item } (the member paid: the invite goes out)
 *   POST   /campaigns/invites/{id}/shipped { note? } → { item }
 *   GET    /campaigns/{id}/claims          a giveaway's claims (#66, Giveaways) { counts, physical: [{ id, email, status, ship_* }] }
 *   POST   /campaigns/claims/{id}/shipped  a physical giveaway gift is sent → the claims of its campaign
 */
final class CampaignsController
{
    public function __construct(
        private readonly CampaignStore $campaigns,
        private readonly Invites $invites,
        private readonly Giveaways $giveaways,
    ) {
    }

    public function list(Request $request, Response $response): Response
    {
        return Json::response($response, 200, ['status' => 'success', 'items' => $this->campaigns->list()]);
    }

    public function create(Request $request, Response $response): Response
    {
        $admin = CurrentSession::of($request)->requireUser();
        return Json::response($response, 201, ['status' => 'success', 'item' => $this->campaigns->create($admin, JsonBody::of($request))]);
    }

    /** @param array{id: string} $args */
    public function get(Request $request, Response $response, array $args): Response
    {
        return Json::response($response, 200, ['status' => 'success', 'item' => $this->campaigns->get($args['id'])]);
    }

    /** @param array{id: string} $args */
    public function update(Request $request, Response $response, array $args): Response
    {
        return Json::response($response, 200, ['status' => 'success', 'item' => $this->campaigns->update($args['id'], JsonBody::of($request))]);
    }

    /** @param array{id: string} $args */
    public function delete(Request $request, Response $response, array $args): Response
    {
        $this->campaigns->delete($args['id']);
        return Json::response($response, 200, ['status' => 'success']);
    }

    /** @param array{id: string} $args */
    public function start(Request $request, Response $response, array $args): Response
    {
        return Json::response($response, 200, ['status' => 'success', 'item' => $this->campaigns->setStatus($args['id'], 'started')]);
    }

    /** @param array{id: string} $args */
    public function stop(Request $request, Response $response, array $args): Response
    {
        return Json::response($response, 200, ['status' => 'success', 'item' => $this->campaigns->setStatus($args['id'], 'stopped')]);
    }

    /** @param array{id: string} $args */
    public function accounts(Request $request, Response $response, array $args): Response
    {
        $q = $request->getQueryParams();
        $filters = array_filter(array_intersect_key($q, array_flip(AccountFilters::NAMES)), 'is_string');
        $result = $this->campaigns->accounts($args['id'], is_string($q['q'] ?? null) ? trim($q['q']) : '', (int) ($q['page'] ?? 1), (int) ($q['page_size'] ?? 25), $filters);
        return Json::response($response, 200, ['status' => 'success'] + $result);
    }

    /** @param array{id: string} $args */
    public function setAccounts(Request $request, Response $response, array $args): Response
    {
        return Json::response($response, 200, ['status' => 'success'] + $this->campaigns->setAccounts($args['id'], JsonBody::of($request)));
    }

    /** @param array{id: string} $args */
    public function invites(Request $request, Response $response, array $args): Response
    {
        $status = $request->getQueryParams()['status'] ?? null;
        return Json::response($response, 200, ['status' => 'success', 'items' => $this->invites->forCampaign($args['id'], is_string($status) && $status !== '' ? $status : null)]);
    }

    /** @param array{id: string} $args */
    public function stats(Request $request, Response $response, array $args): Response
    {
        return Json::response($response, 200, ['status' => 'success'] + $this->invites->stats($args['id']));
    }

    /** @param array{id: string} $args */
    public function claims(Request $request, Response $response, array $args): Response
    {
        return Json::response($response, 200, ['status' => 'success'] + $this->giveaways->forCampaign($args['id']));
    }

    /** @param array{id: string} $args */
    public function claimShipped(Request $request, Response $response, array $args): Response
    {
        $campaign = $this->giveaways->markShipped((int) $args['id']);
        return Json::response($response, 200, ['status' => 'success'] + $this->giveaways->forCampaign($campaign));
    }

    /** @param array{id: string} $args */
    public function paid(Request $request, Response $response, array $args): Response
    {
        $admin = CurrentSession::of($request)->requireUser();
        return Json::response($response, 200, ['status' => 'success', 'item' => $this->invites->markPaid($admin, $args['id'])]);
    }

    /** @param array{id: string} $args */
    public function shipped(Request $request, Response $response, array $args): Response
    {
        $admin = CurrentSession::of($request)->requireUser();
        $note = JsonBody::of($request)['note'] ?? null;
        return Json::response($response, 200, ['status' => 'success', 'item' => $this->invites->markShipped($admin, $args['id'], is_string($note) ? $note : null)]);
    }
}
