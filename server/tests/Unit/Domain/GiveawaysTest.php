<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Domain\Accounts\ShippingAddress;
use Scutoons\Domain\Accounts\Users;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Auth\LoginService;
use Scutoons\Domain\Campaigns\CampaignStore;
use Scutoons\Domain\Campaigns\Giveaways;
use Scutoons\Domain\Catalog\CatalogAdmin;
use Scutoons\Domain\Newsletter\Campaigns;
use Scutoons\Domain\Newsletter\Contacts;
use Scutoons\Domain\Newsletter\NewsletterRenderer;
use Scutoons\Domain\Newsletter\NewsletterSender;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Mail\Transport;
use Scutoons\Tests\TestCase;

/**
 * Giveaway campaigns – "Verschenken" (Campaigns\Giveaways, #66, agents/research/CAMPAIGN-GIFTS.md): a newsletter's
 * gift button, claimed once per address while the campaign runs and its stock lasts, the item's own per-account limit,
 * email-only recipients sign up with a personal promo code, physical gifts wait for the account's address
 */
final class GiveawaysTest extends TestCase
{
    /** @var array<string, string> recipient → the mail's text */
    private array $texts = [];
    /** @var array<string, mixed> the admin who makes the campaigns (an account: it gets the newsletter too) */
    private array $admin = [];

    protected function settings(): array
    {
        return ['NEWSLETTER_DELAY_MS' => '0', 'API_URL' => 'http://api.test', 'APP_URL' => 'https://app.test'] + parent::settings();
    }

    protected function setUp(): void
    {
        parent::setUp();
        $this->admin = $this->user('boss@example.com');
    }

    /**
     * A started giveaway campaign (30 days)
     * @param array<string, mixed> $fields
     */
    private function giveaway(array $fields = []): string
    {
        $store = $this->get(CampaignStore::class);
        $id = (string) $store->create($this->admin, $fields + [
            'kind' => 'giveaway', 'name' => 'Gift', 'item_id' => 'record_player',
            'starts_at' => gmdate('Y-m-d H:i:s', self::NOW - 60), 'ends_at' => gmdate('Y-m-d H:i:s', self::NOW + 30 * 86400),
        ])['id'];
        $store->setStatus($id, 'started');
        return $id;
    }

    /**
     * Sends a newsletter with the gift of $campaign to every member and contact
     * @return array<string, array{signin: string, gift: string}> recipient → the tokens of its gift link ('' = none)
     */
    private function send(string $campaign): array
    {
        $transport = new FakeNewsletterTransport(Settings::fixed(self::root(), $this->settings()), []);
        $this->container->set(Transport::class, $transport);
        $id = (int) $this->get(Campaigns::class)->create([
            'title' => 'A gift', 'audiences' => ['members_all', 'contacts'], 'gift_campaign_id' => $campaign,
            'subject_de' => 'Hallo', 'subject_en' => 'Hi', 'body_de' => "Für dich:\n\n{gift}", 'body_en' => "For you:\n\n{gift}",
        ], 'admin:x')['id'];
        $this->get(Campaigns::class)->start($id, 'admin:x');
        $this->get(NewsletterSender::class)->batch();
        $links = [];
        foreach ($transport->sent as $mail) {
            preg_match('#\?signin=([A-Za-z0-9_-]+)#', $mail['text'], $s);
            preg_match('#[?&]gift=([A-Za-z0-9_.-]+)#', $mail['text'], $g);
            $links[$mail['to']] = ['signin' => $s[1] ?? '', 'gift' => $g[1] ?? ''];
            $this->texts[$mail['to']] = $mail['text'];
        }
        return $links;
    }

    private static function errorCode(callable $work): ?string
    {
        try {
            $work();
        } catch (ApiError $error) {
            return $error->errorCode;
        }
        return null;
    }

    /** @return array<string, mixed> */
    private function account(string $email): array
    {
        return $this->get(Users::class)->byEmail($email) ?? throw new \RuntimeException("no account $email");
    }

    public function testMembersQrOwnersAndEmailOnlyRecipientsEachClaimOnceWhileTheStockLasts(): void
    {
        $member = $this->user('member@example.com');
        $code = $this->code('membership', ['email' => 'qr@example.com']);
        $this->get(Contacts::class)->import("qr@example.com, en\nevent@example.com, de", 'event:Leipzig', null, null, false, 'admin:x');
        $until = gmdate('Y-m-d H:i:s', self::NOW + 90 * 86400);
        $campaign = $this->giveaway(['stock' => 3, 'access' => 'until', 'access_until' => $until]);
        $gifts = $this->get(Giveaways::class);
        $emails = ['member@example.com', 'boss@example.com', 'qr@example.com', 'event@example.com'];
        self::assertSame(['members' => 2, 'product_code' => 1, 'email_only' => 1], $gifts->reach($emails), 'what the editor shows');

        $links = $this->send($campaign);
        self::assertStringContainsString('A gift is waiting for you: https://app.test/en/?signin=', $this->texts['member@example.com']);
        self::assertStringContainsString('Ein Geschenk wartet auf dich: https://app.test/?signin=', $this->texts['event@example.com'], 'email only: a sign-up link too');
        $promo = (string) $this->db->value('SELECT code FROM campaign_codes WHERE campaign_id = ? AND email = ?', [$campaign, 'event@example.com']);
        self::assertSame('promo', $this->db->value('SELECT type FROM qr WHERE code = ?', [$promo]));
        self::assertSame(1, (int) $this->db->value('SELECT COUNT(*) FROM campaign_codes'), 'codes only for the email-only recipient');

        self::assertSame(['open', 'record_player', 3], [($i = $gifts->info($links['member@example.com']['gift'], null, 'de'))['state'], $i['item']['id'], $i['left']]);
        self::assertSame('other_account', $gifts->info($links['member@example.com']['gift'], $this->admin, 'de')['state']);
        self::assertSame('other_account', self::errorCode(fn() => $gifts->claim($this->admin, $links['member@example.com']['gift'], 'de')));

        self::assertSame(['claimed', 2], [($c = $gifts->claim($member, $links['member@example.com']['gift'], 'de'))['claim']['status'], $c['left']]);
        self::assertSame(1, (int) $this->db->value("SELECT COUNT(*) FROM inventory WHERE user_id = ? AND item_id = 'record_player' AND source = 'gift'", [$member['id']]));
        self::assertSame('already_claimed', self::errorCode(fn() => $gifts->claim($member, $links['member@example.com']['gift'], 'de')));

        // the QR owner signs up with the product code, then claims
        self::assertSame([$code], $this->get(LoginService::class)->verify(['token' => $links['qr@example.com']['signin']])['claimed']);
        self::assertSame(1, $gifts->claim($this->account('qr@example.com'), $links['qr@example.com']['gift'], 'en')['left']);
        // the email-only one signs up with the personal promo code: it gets the campaign's reading rule
        self::assertTrue($this->get(LoginService::class)->verify(['token' => $links['event@example.com']['signin']])['created']);
        self::assertSame(date('Y-m-d H:i:s', (int) strtotime($until . ' UTC')), $this->db->value('SELECT expires_at FROM qr WHERE code = ?', [$promo]));
        self::assertSame(0, $gifts->claim($this->account('event@example.com'), $links['event@example.com']['gift'], 'de')['left']);
        // the promo code signs up only its address
        self::assertSame('used', $this->get(\Scutoons\Domain\Codes\CodeService::class)->signupRefusal($promo, 'someone@example.com'));
        // the stock is gone
        self::assertSame('sold_out', self::errorCode(fn() => $gifts->claim($this->admin, $links['boss@example.com']['gift'], 'de')));
    }

    public function testTheWindowAndTheItemsOwnLimit(): void
    {
        $this->get(CatalogAdmin::class)->put('items', 'one_badge', ['id' => 'one_badge', 'name' => ['en' => 'Badge'], 'kind' => 'virtual',
            'category' => 'tool', 'icon' => 'badge.png', 'stackable' => false, 'starter' => false, 'grantable' => ['story'], 'slot' => null, 'skills' => [], 'max' => 1], 'test');
        $member = $this->user('member@example.com');
        $this->db->run("INSERT INTO inventory (id, user_id, item_id, variant, qty, source, acquired_at, updated_at) VALUES (UUID(), ?, 'one_badge', NULL, 1, 'grant', ?, ?)",
            [$member['id'], $this->clock->utc(), $this->clock->utc()]);
        $links = $this->send($this->giveaway(['item_id' => 'one_badge']));
        $gifts = $this->get(Giveaways::class);
        self::assertSame('max_reached', self::errorCode(fn() => $gifts->claim($member, $links['member@example.com']['gift'], 'de')));
        $this->clock->advance(31 * 86400);
        self::assertSame('ended', $gifts->info($links['member@example.com']['gift'], $member, 'de')['state']);
        self::assertSame('ended', self::errorCode(fn() => $gifts->claim($member, $links['member@example.com']['gift'], 'de')));
    }

    public function testAPhysicalGiftWaitsForTheAccountsAddressThenShipsOrLapses(): void
    {
        $member = $this->user('member@example.com');
        $campaign = $this->giveaway(['item_id' => 'dago_hoodie', 'variant' => 'black', 'countries' => ['DE', 'AT'], 'stock' => 5]);
        $links = $this->send($campaign);
        $gifts = $this->get(Giveaways::class);
        $claimed = $gifts->claim($member, $links['member@example.com']['gift'], 'de');
        self::assertSame(['address', true], [$claimed['claim']['status'], $claimed['item']['physical']]);
        self::assertSame(0, (int) $this->db->value("SELECT COUNT(*) FROM inventory WHERE user_id = ? AND item_id = 'dago_hoodie'", [$member['id']]), 'sent, not added');
        self::assertCount(1, $gifts->waitingForAddress((string) $member['id'], 'de'));

        $shipping = $this->get(ShippingAddress::class);
        self::assertSame('invalid_country', self::errorCode(fn() => $shipping->set((string) $member['id'], ['name' => 'Ana', 'address' => 'Weg 1', 'country' => 'Germany'])));
        $shipping->set((string) $member['id'], ['name' => 'Ana', 'address' => "Weg 1\n10115 Berlin", 'country' => 'ch']);
        self::assertSame(0, $gifts->addressGiven((string) $member['id']), 'it does not ship to CH');
        $shipping->set((string) $member['id'], ['name' => 'Ana', 'address' => "Weg 1\n10115 Berlin", 'country' => 'de']);
        self::assertSame(1, $gifts->addressGiven((string) $member['id']));
        $list = $gifts->forCampaign($campaign);
        self::assertSame(['ship', 'Ana', 'DE'], [$list['physical'][0]['status'], $list['physical'][0]['ship_name'], $list['physical'][0]['ship_country']]);
        self::assertSame(1, $this->get(CampaignStore::class)->get($campaign)['counts']['to_ship']);
        self::assertSame($campaign, $gifts->markShipped($list['physical'][0]['id']));
        self::assertSame('not_ready', self::errorCode(fn() => $gifts->markShipped($list['physical'][0]['id'])));

        // the admin's own claim never gets an address: 30 days after the campaign's end it lapses (the piece is free again)
        $gifts->claim($this->admin, $links['boss@example.com']['gift'], 'de');
        self::assertSame(3, $this->get(CampaignStore::class)->get($campaign)['stock_left']);
        $this->clock->advance((30 + Giveaways::LAPSE_DAYS) * 86400 + 60);
        self::assertSame(['shipped' => 1, 'lapsed' => 1], array_filter($gifts->forCampaign($campaign)['counts']));
        self::assertSame(4, $this->get(CampaignStore::class)->get($campaign)['stock_left']);
    }

    public function testTheNewsletterChecksItsGiftAndThePreviewShowsTheButton(): void
    {
        $this->user('member@example.com');
        $campaigns = $this->get(Campaigns::class);
        $id = (int) $campaigns->create(['title' => 'T', 'audiences' => ['members_all'], 'subject_de' => 'S', 'subject_en' => 'S',
            'body_de' => "A\n\n{gift}", 'body_en' => 'B'], 'admin:x')['id'];
        self::assertSame('gift_missing', self::errorCode(fn() => $campaigns->start($id, 'admin:x')));
        $invite = (string) $this->get(CampaignStore::class)->create($this->admin, ['name' => 'Invite'])['id'];
        self::assertSame('invalid_gift', self::errorCode(fn() => $campaigns->update($id, ['gift_campaign_id' => $invite], 'admin:x')), 'an Einladen campaign is no gift');
        // a giveaway is made before its item is chosen; it needs one to start
        $store = $this->get(CampaignStore::class);
        $draft = (string) $store->create($this->admin, ['kind' => 'giveaway', 'name' => 'Later'])['id'];
        self::assertSame('invalid_item', self::errorCode(fn() => $store->setStatus($draft, 'started')));
        $giveaway = $this->giveaway();
        self::assertSame($giveaway, $campaigns->update($id, ['gift_campaign_id' => $giveaway], 'admin:x')['gift_campaign_id']);
        $this->get(CampaignStore::class)->setStatus($giveaway, 'stopped');
        self::assertSame('gift_ended', self::errorCode(fn() => $campaigns->start($id, 'admin:x')));
        self::assertNull($campaigns->update($id, ['gift_campaign_id' => null], 'admin:x')['gift_campaign_id']);

        $mail = $this->get(NewsletterRenderer::class)->render(['subject' => 'S', 'body' => "A\n\n{gift}"], 'de', 'x@example.com', null);
        self::assertStringContainsString('Ein Geschenk wartet auf dich: https://app.test/', $mail['text']);
    }
}
