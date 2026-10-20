<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\Accounts\ShippingAddress;
use Scutoons\Domain\Campaigns\Giveaways;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;

/**
 * A giveaway's gift (#66, Domain\Campaigns\Giveaways; agents/research/CAMPAIGN-GIFTS.md) and the account's shipping
 * address it may need:
 *   GET  /api/gift?t=&locale=     { gift: { state, item, starts_at, until, left, claim } } – no session needed (the
 *                                 state `other_account` only when signed in with another address)
 *   POST /api/gift                { t, locale? } → { gift } – signed in
 *   GET  /api/me/shipping         { address: { name, address, country } | null, waiting: [{ id, item, countries, until }] }
 *   PUT  /api/me/shipping         { name, address, country } (all empty = remove) → the same; waiting gifts whose
 *                                 countries fit move on to "to send"
 */
final class CampaignGiftsController
{
    public function __construct(private readonly Giveaways $giveaways, private readonly ShippingAddress $shipping)
    {
    }

    public function info(Request $request, Response $response): Response
    {
        $query = $request->getQueryParams();
        $user = CurrentSession::of($request)->user();
        return Json::response($response, 200, ['status' => 'success', 'gift' => $this->giveaways->info($query['t'] ?? null, $user, self::locale($query['locale'] ?? null, $user))]);
    }

    public function claim(Request $request, Response $response): Response
    {
        $body = JsonBody::of($request);
        $user = CurrentSession::of($request)->requireUser();
        return Json::response($response, 200, ['status' => 'success', 'gift' => $this->giveaways->claim($user, $body['t'] ?? null, self::locale($body['locale'] ?? null, $user))]);
    }

    public function shipping(Request $request, Response $response): Response
    {
        $user = CurrentSession::of($request)->requireUser();
        return Json::response($response, 200, ['status' => 'success'] + $this->mine($user));
    }

    public function setShipping(Request $request, Response $response): Response
    {
        $body = JsonBody::of($request);
        $user = CurrentSession::of($request)->requireUser();
        $this->shipping->set((string) $user['id'], $body);
        $this->giveaways->addressGiven((string) $user['id']);
        return Json::response($response, 200, ['status' => 'success'] + $this->mine($user));
    }

    /**
     * @param array<string, mixed> $user
     * @return array{address: ?array<string, string>, waiting: list<array<string, mixed>>}
     */
    private function mine(array $user): array
    {
        $userId = (string) $user['id'];
        return ['address' => $this->shipping->of($userId), 'waiting' => $this->giveaways->waitingForAddress($userId, self::locale(null, $user))];
    }

    /** @param array<string, mixed>|null $user */
    private static function locale(mixed $asked, ?array $user): string
    {
        if ($asked === 'de' || $asked === 'en') return $asked;
        return ($user['locale'] ?? null) === 'en' ? 'en' : 'de';
    }
}
