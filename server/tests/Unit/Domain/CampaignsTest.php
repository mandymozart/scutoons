<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Auth\LoginService;
use Scutoons\Domain\Campaigns\CampaignStore;
use Scutoons\Domain\Campaigns\Invites;
use Scutoons\Domain\Inventory\InventoryService;
use Scutoons\Domain\Ownership\Gifts;
use Scutoons\Domain\Ownership\OwnershipTransfers;
use Scutoons\Domain\RequestContext;
use Scutoons\Tests\TestCase;

/**
 * Gift invite campaigns (agents/research/GIFT-INVITES.md, FEATURES 10.96): the campaign's rules, an invite with the
 * member's own item or the campaign's item, recall, single-use codes, delivery at signup with the reading rule, the
 * return after the deadline, a physical gift the member pays (payment by hand, address, shipping), the numbers.
 */
final class CampaignsTest extends TestCase
{
    /** @return array<string, string> */
    protected function settings(): array
    {
        return ['MAIL_ADMIN_TO' => 'hello@example.com'] + parent::settings();
    }

    protected function setUp(): void
    {
        parent::setUp();
        $this->get(RequestContext::class)->set('203.0.113.9', 'device-unit-test', 'http://viewer.test');
    }

    // ── The campaign ─────────────────────────────────────────────────────────────────────────────────────────

    public function testCampaignRulesAndWhoMayInvite(): void
    {
        $store = $this->get(CampaignStore::class);
        $admin = $this->user('admin@example.com');
        $this->assertApiError(400, 'invalid_name', fn() => $store->create($admin, []));
        $this->assertApiError(400, 'invalid_window', fn() => $store->create($admin, ['name' => 'X', 'starts_at' => '2026-11-02', 'ends_at' => '2026-11-01']));
        $this->assertApiError(400, 'invalid_item', fn() => $store->create($admin, ['name' => 'X', 'gift' => 'campaign']));
        $this->assertApiError(400, 'invalid_payer', fn() => $store->create($admin, ['name' => 'X', 'gift' => 'campaign', 'item_id' => 'awakening_vinyl']));
        $this->assertApiError(400, 'invalid_countries', fn() => $store->create($admin, ['name' => 'X', 'gift' => 'campaign', 'item_id' => 'awakening_vinyl', 'payer' => 'scutoons']));
        $this->assertApiError(400, 'invalid_access', fn() => $store->create($admin, ['name' => 'X', 'access' => 'until']));

        $c = $this->campaign();
        $anna = $this->user('anna@example.com');
        $ben = $this->user('ben-member@example.com');
        // chosen accounts only
        self::assertNull($store->offerFor($anna));
        $store->setAccounts($c, ['add' => [$anna['id']]]);
        self::assertSame($c, $store->offerFor($anna)['id'] ?? null);
        self::assertNull($store->offerFor($ben));
        // "all" ticks every account (also later ones); unticking one leaves "all" for the list of the others
        $page = $store->setAccounts($c, ['all' => true]);
        self::assertSame('all', $page['mode']);
        self::assertNotNull($store->offerFor($this->user('later@example.com')));
        $store->setAccounts($c, ['remove' => [$ben['id']]]);
        self::assertNull($store->offerFor($ben));
        self::assertNotNull($store->offerFor($anna));
        self::assertSame('chosen', $store->accounts($c)['mode']);
        // stopped / outside the window: nothing
        $store->setStatus($c, 'stopped');
        self::assertNull($store->offerFor($anna));
        $store->setStatus($c, 'started');
        $this->clock->advance(40 * 86400);
        self::assertNull($store->offerFor($anna), 'ended');
    }

    public function testTheGiftFormOffersAnInviteOnlyWhileACampaignRuns(): void
    {
        $anna = $this->user('anna@example.com');
        $this->get(InventoryService::class)->add((string) $anna['id'], 'cucumber', null, 3, 'story');
        $gifts = $this->get(Gifts::class);
        $error = $this->assertApiError(404, 'recipient_not_found', fn() => $gifts->gift($anna, ['item_id' => 'cucumber', 'to' => 'ben@example.com']));
        self::assertArrayNotHasKey('invite', $error->extra);

        $c = $this->campaign(['inviters' => 'all']);
        $error = $this->assertApiError(404, 'recipient_not_found', fn() => $gifts->gift($anna, ['item_id' => 'cucumber', 'to' => 'ben@example.com']));
        self::assertSame([$c, 'own', 5], [$error->extra['invite']['id'], $error->extra['invite']['gift'], $error->extra['invite']['left']]);
    }

    // ── An invite with the member's own item ─────────────────────────────────────────────────────────────────

    public function testInviteRecallAndSingleUse(): void
    {
        $this->campaign(['inviters' => 'all']);
        $anna = $this->user('anna@example.com');
        $this->user('member@example.com');
        $inventory = $this->get(InventoryService::class);
        $inventory->add((string) $anna['id'], 'cucumber', null, 3, 'story');
        $invites = $this->get(Invites::class);

        $this->assertApiError(409, 'has_account', fn() => $invites->create($anna, ['item_id' => 'cucumber', 'to' => 'member@example.com']));
        $invite = $invites->create($anna, ['item_id' => 'cucumber', 'qty' => 2, 'to' => 'Ben@Example.com']);
        self::assertSame(['waiting', 'ben@example.com', 2], [$invite['status'], $invite['email'], $invite['qty']]);
        self::assertSame(1, $inventory->count((string) $anna['id'], 'cucumber'), 'the gift left the inventory');
        self::assertSame(['invite_received'], $this->mails('ben@example.com'));
        $code = $this->inviteCode($invite['id']);
        self::assertSame('promo', $this->db->value('SELECT type FROM qr WHERE code = ?', [$code]), 'an ordinary promo code');
        // one open invite per address
        $this->assertApiError(409, 'already_invited', fn() => $invites->create($anna, ['item_id' => 'cucumber', 'to' => 'ben@example.com']));

        $recalled = $invites->recall($anna, $invite['id']);
        self::assertSame('recalled', $recalled['status']);
        self::assertSame(3, $inventory->count((string) $anna['id'], 'cucumber'), 'back');
        self::assertNotNull($this->db->value('SELECT revoked_at FROM qr WHERE code = ?', [$code]));
        $this->assertApiError(409, 'not_open', fn() => $invites->recall($anna, $invite['id']));
        // the code signs nobody up any more
        $login = $this->get(LoginService::class);
        $login->request(['email' => 'ben@example.com', 'code' => $code]);
        $this->assertApiError(400, 'invalid_otp', fn() => $login->verify(['email' => 'ben@example.com', 'otp' => '000000']));
    }

    public function testSignupDeliversTheGiftAndKeepsReadingAfterTheCampaign(): void
    {
        $this->campaign(['inviters' => 'all', 'access' => 'keep']);
        $anna = $this->user('anna@example.com');
        $this->get(InventoryService::class)->add((string) $anna['id'], 'cucumber', null, 2, 'story');
        $invite = $this->get(Invites::class)->create($anna, ['item_id' => 'cucumber', 'to' => 'ben@example.com']);
        $code = $this->inviteCode($invite['id']);
        self::assertNotNull($this->db->value('SELECT expires_at FROM qr WHERE code = ?', [$code]), 'the code ends with the invite until used');

        $ben = $this->signup('ben@example.com', $code);
        self::assertSame(1, $this->get(InventoryService::class)->count((string) $ben['id'], 'cucumber'));
        self::assertSame('joined', $this->db->value('SELECT status FROM campaign_invites WHERE id = ?', [$invite['id']]));
        self::assertNull($this->db->value('SELECT expires_at FROM qr WHERE code = ?', [$code]), 'keep: the invitee reads on after the campaign');
        $notice = $this->get(OwnershipTransfers::class)->unseen((string) $ben['id']);
        self::assertSame(['gift', 'cucumber', 'anna@example.com'], [$notice[0]['kind'], $notice[0]['item_id'], $notice[0]['from']], '"anna schenkt dir …"');
        // used once
        $login = $this->get(LoginService::class);
        $login->request(['email' => 'carl@example.com', 'code' => $code]);
        $this->assertApiError(400, 'invalid_otp', fn() => $login->verify(['email' => 'carl@example.com', 'otp' => '000000']));
        $this->assertApiError(409, 'not_open', fn() => $this->get(Invites::class)->recall($anna, $invite['id']));
    }

    public function testUntilADateAndDeliveryByEmailWithAnotherCode(): void
    {
        $this->campaign(['inviters' => 'all', 'access' => 'until', 'access_until' => '2027-01-31 23:00:00']);
        $anna = $this->user('anna@example.com');
        $this->get(InventoryService::class)->add((string) $anna['id'], 'cucumber', null, 2, 'story');
        $first = $this->get(Invites::class)->create($anna, ['item_id' => 'cucumber', 'to' => 'ben@example.com']);
        $code = $this->inviteCode($first['id']);
        $this->signup('ben@example.com', $code);
        self::assertSame(date('Y-m-d H:i:s', (int) strtotime('2027-01-31 23:00:00 UTC')), (string) $this->db->value('SELECT expires_at FROM qr WHERE code = ?', [$code]));

        // dora signs up with a product code: the invite arrives anyway (by email), her code is not touched
        $second = $this->get(Invites::class)->create($anna, ['item_id' => 'cucumber', 'to' => 'dora@example.com']);
        $dora = $this->signup('dora@example.com', $this->code('membership', ['edition_id' => '3_catpope_vinyl', 'edition_number' => 7]));
        self::assertSame('joined', $this->db->value('SELECT status FROM campaign_invites WHERE id = ?', [$second['id']]));
        self::assertSame(1, $this->get(InventoryService::class)->count((string) $dora['id'], 'cucumber'));
        self::assertNotNull($this->db->value('SELECT expires_at FROM qr WHERE code = ?', [$this->inviteCode($second['id'])]), 'the unused invite code keeps its end');
    }

    public function testNotJoinedInTimeTheGiftComesBack(): void
    {
        $this->campaign(['inviters' => 'all', 'wait_days' => 3]);
        $anna = $this->user('anna@example.com');
        $inventory = $this->get(InventoryService::class);
        $inventory->add((string) $anna['id'], 'cucumber', null, 1, 'story');
        $invite = $this->get(Invites::class)->create($anna, ['item_id' => 'cucumber', 'to' => 'ben@example.com']);
        self::assertSame(0, $inventory->count((string) $anna['id'], 'cucumber'));
        $this->clock->advance(3 * 86400 + 1);
        self::assertSame(['returned' => 1, 'unpaid' => 0], $this->get(Invites::class)->sweep());
        self::assertSame(1, $inventory->count((string) $anna['id'], 'cucumber'));
        self::assertSame(['invite_returned'], $this->mails('anna@example.com'));
        // too late to sign up with it: the code ran out with the invite
        $login = $this->get(LoginService::class);
        $login->request(['email' => 'ben@example.com', 'code' => $this->inviteCode($invite['id'])]);
        $this->assertApiError(400, 'invalid_otp', fn() => $login->verify(['email' => 'ben@example.com', 'otp' => '000000']));
    }

    public function testOwnPiecesWithACodeNeedAnAccountAndLimitsHold(): void
    {
        $this->campaign(['inviters' => 'all', 'per_member' => 1]);
        $anna = $this->user('anna@example.com');
        $code = $this->code('membership', ['edition_id' => '3_catpope_vinyl', 'edition_number' => 2]);
        $this->get(\Scutoons\Domain\Codes\CodeService::class)->claim($anna, $code, null, null);
        $entry = (string) $this->db->value('SELECT id FROM inventory WHERE code = ?', [$code]);
        $invites = $this->get(Invites::class);
        $this->assertApiError(400, 'code_needs_account', fn() => $invites->create($anna, ['entry_id' => $entry, 'to' => 'ben@example.com']));
        $this->get(InventoryService::class)->add((string) $anna['id'], 'cucumber', null, 2, 'story');
        $invites->create($anna, ['item_id' => 'cucumber', 'to' => 'ben@example.com']);
        $this->assertApiError(429, 'campaign_limit', fn() => $invites->create($anna, ['item_id' => 'cucumber', 'to' => 'carl@example.com']));
        $this->db->run("UPDATE campaigns SET status = 'stopped'");
        $this->assertApiError(403, 'no_campaign', fn() => $invites->create($anna, ['item_id' => 'cucumber', 'to' => 'carl@example.com']));
    }

    public function testAContainerGoesOnlyToAnAccount(): void
    {
        $this->get(\Scutoons\Domain\Catalog\CatalogAdmin::class)->put('items', 'songbook_small', ['id' => 'songbook_small', 'name' => ['en' => 'Songbook'], 'kind' => 'virtual',
            'category' => 'tool', 'icon' => 'songbook.png', 'stackable' => false, 'starter' => false, 'grantable' => ['story'], 'slot' => null, 'skills' => [],
            'holds' => ['categories' => ['score'], 'max' => 2]], 'test');
        $this->campaign(['inviters' => 'all']);
        $anna = $this->user('anna@example.com');
        $this->get(InventoryService::class)->add((string) $anna['id'], 'songbook_small', null, 1, 'story');
        $this->assertApiError(400, 'container_needs_account', fn() => $this->get(Invites::class)->create($anna, ['item_id' => 'songbook_small', 'to' => 'ben@example.com']));
        self::assertSame(1, $this->get(InventoryService::class)->count((string) $anna['id'], 'songbook_small'), 'it stays');
    }

    // ── The campaign's item, physical, paid by the member ────────────────────────────────────────────────────

    public function testPhysicalGiftPaidByHandAddressAndShipping(): void
    {
        $c = $this->campaign(['inviters' => 'all', 'gift' => 'campaign', 'item_id' => 'awakening_vinyl', 'payer' => 'member', 'price_cents' => 3900, 'countries' => 'AT,DE', 'stock' => 2]);
        $anna = $this->user('anna@example.com');
        $admin = $this->user('admin@example.com');
        $invites = $this->get(Invites::class);

        $invite = $invites->create($anna, ['to' => 'ben@example.com']);
        self::assertSame(['payment', 'campaign', true, 3900], [$invite['status'], $invite['gift'], $invite['physical'], $invite['price_cents']]);
        self::assertSame(['invite_payment_requested'], $this->mails('hello@example.com'));
        self::assertSame([], $this->mails('ben@example.com'), 'nothing to the friend before the payment');
        self::assertSame(1, $this->get(CampaignStore::class)->get($c)['stock_left']);

        $paid = $invites->markPaid($admin, $invite['id']);
        self::assertSame('waiting', $paid['status']);
        self::assertSame(['invite_received'], $this->mails('ben@example.com'));
        $this->assertApiError(409, 'not_payment', fn() => $invites->markPaid($admin, $invite['id']));

        $ben = $this->signup('ben@example.com', $this->inviteCode($invite['id']));
        self::assertSame(0, $this->get(InventoryService::class)->count((string) $ben['id'], 'awakening_vinyl'), 'the real thing comes by post – its QR code claims it');
        $mine = $invites->mine($ben);
        self::assertSame([$invite['id'], ['AT', 'DE']], [$mine['address_needed'][0]['id'], $mine['address_needed'][0]['countries']]);
        $this->assertApiError(409, 'not_shippable', fn() => $invites->markShipped($admin, $invite['id'], null));
        $this->assertApiError(400, 'invalid_country', fn() => $invites->setAddress($ben, $invite['id'], ['name' => 'Ben', 'address' => "Gasse 1\n1010 Wien", 'country' => 'FR']));
        $mine = $invites->setAddress($ben, $invite['id'], ['name' => 'Ben', 'address' => "Gasse 1\n1010 Wien", 'country' => 'at']);
        self::assertSame([], $mine['address_needed']);
        $toShip = $invites->forCampaign($c, 'to_ship');
        self::assertSame(['AT', "Gasse 1\n1010 Wien"], [$toShip[0]['ship']['country'], $toShip[0]['ship']['address']]);
        $shipped = $invites->markShipped($admin, $invite['id'], 'DHL 123');
        self::assertTrue($shipped['shipped']);
        self::assertSame([], $invites->forCampaign($c, 'to_ship'));
        $this->assertApiError(409, 'shipped', fn() => $invites->setAddress($ben, $invite['id'], ['name' => 'Ben', 'address' => 'x', 'country' => 'DE']));
    }

    public function testUnpaidEndsAndStockRunsOut(): void
    {
        $c = $this->campaign(['inviters' => 'all', 'gift' => 'campaign', 'item_id' => 'cucumber', 'stock' => 1, 'wait_days' => 2]);
        $anna = $this->user('anna@example.com');
        $invites = $this->get(Invites::class);
        $invites->create($anna, ['to' => 'ben@example.com']);
        $this->assertApiError(410, 'sold_out', fn() => $invites->create($anna, ['to' => 'carl@example.com']));
        self::assertSame(0, ($this->get(CampaignStore::class)->offerFor($anna) ?? [])['left'] ?? null);

        $paidCampaign = $this->campaign(['name' => 'Vinyl', 'inviters' => 'all', 'gift' => 'campaign', 'item_id' => 'awakening_vinyl', 'payer' => 'member', 'price_cents' => 100,
            'countries' => 'AT', 'wait_days' => 2, 'ends_at' => '2026-12-31 00:00:00']);
        unset($paidCampaign);
        $this->db->run("UPDATE campaigns SET status = 'stopped' WHERE id = ?", [$c]); // the vinyl campaign is the running one now
        $payment = $invites->create($anna, ['to' => 'dora@example.com']);
        self::assertSame('payment', $payment['status']);
        $this->clock->advance(2 * 86400 + 1);
        self::assertSame(1, $invites->sweep()['unpaid']);
        self::assertSame('unpaid', $this->db->value('SELECT status FROM campaign_invites WHERE id = ?', [$payment['id']]));
        self::assertContains('invite_returned', $this->mails('anna@example.com'));
    }

    public function testStatisticsCountWhoInvitedAndWhoJoined(): void
    {
        $c = $this->campaign(['inviters' => 'all', 'wait_days' => 2]);
        $anna = $this->user('anna@example.com');
        $max = $this->user('max@example.com');
        $inventory = $this->get(InventoryService::class);
        $inventory->add((string) $anna['id'], 'cucumber', null, 5, 'story');
        $inventory->add((string) $max['id'], 'cucumber', null, 5, 'story');
        $invites = $this->get(Invites::class);
        $joins = $invites->create($anna, ['item_id' => 'cucumber', 'to' => 'ben@example.com']);
        $invites->create($anna, ['item_id' => 'cucumber', 'to' => 'carl@example.com']);
        $later = $invites->create($max, ['item_id' => 'cucumber', 'to' => 'dora@example.com']);
        $this->signup('ben@example.com', $this->inviteCode($joins['id']));
        $this->clock->advance(2 * 86400 + 1);
        $invites->sweep(); // carl and dora: back
        $this->signup('dora@example.com', $this->code('membership', ['edition_id' => '3_catpope_vinyl', 'edition_number' => 8])); // joins later, without the invite
        unset($later);

        $stats = $invites->stats($c);
        self::assertSame([2, 3, 1, 2, 1], [$stats['inviters'], $stats['sent'], $stats['by_status']['joined'], $stats['by_status']['returned'], $stats['joined_later']]);
        $byEmail = array_column($stats['members'], null, 'email');
        self::assertSame([2, 1, 0], [$byEmail['anna@example.com']['invites'], $byEmail['anna@example.com']['joined'], $byEmail['anna@example.com']['joined_later']]);
        self::assertSame(1, $byEmail['max@example.com']['joined_later']);
    }

    // ── helpers ──────────────────────────────────────────────────────────────────────────────────────────────

    /**
     * a started campaign running now (NOW … +30 days)
     * @param array<string, mixed> $fields
     */
    private function campaign(array $fields = []): string
    {
        $store = $this->get(CampaignStore::class);
        $admin = $this->user('campaign-admin-' . bin2hex(random_bytes(3)) . '@example.com');
        $c = $store->create($admin, $fields + ['name' => 'Advent invites', 'starts_at' => $this->clock->utc(-60), 'ends_at' => $this->clock->utc(30 * 86400)]);
        if (($fields['inviters'] ?? null) === 'all') $store->setAccounts($c['id'], ['all' => true]);
        $store->setStatus($c['id'], 'started');
        return $c['id'];
    }

    private function inviteCode(string $inviteId): string
    {
        return (string) $this->db->value('SELECT invite_code FROM campaign_invites WHERE id = ?', [$inviteId]);
    }

    /**
     * signs up through LoginService (request + the mailed one-time code)
     * @return array<string, mixed> the new users row
     */
    private function signup(string $email, string $code): array
    {
        $login = $this->get(LoginService::class);
        $login->request(['email' => $email, 'code' => $code]);
        $result = $login->verify(['email' => $email, 'otp' => $this->otp()]);
        self::assertTrue($result['created'], "account for $email");
        return (array) $this->db->one('SELECT * FROM users WHERE email = ?', [$email]);
    }

    private function otp(): string
    {
        $log = (string) file_get_contents($this->dir . '/mail.log');
        $last = (string) substr($log, (int) strrpos($log, '=== '));
        return preg_match('/^\s{4}(\d{6})\s*$/m', $last, $m) === 1 ? $m[1] : '';
    }

    /** @return list<string> */
    private function mails(string $to): array
    {
        return array_map('strval', array_column($this->db->all('SELECT template FROM mail_log WHERE recipient = ? ORDER BY id', [$to]), 'template'));
    }

    private function assertApiError(int $status, string $code, callable $call): ApiError
    {
        try {
            $call();
        } catch (ApiError $error) {
            self::assertSame([$status, $code], [$error->status, $error->errorCode], $error->getMessage());
            return $error;
        }
        self::fail("expected ApiError $status $code");
    }
}
