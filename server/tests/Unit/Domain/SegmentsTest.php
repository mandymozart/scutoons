<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Domain\Admin\AccountFilters;
use Scutoons\Domain\Admin\Batches;
use Scutoons\Domain\Admin\CodeAdmin;
use Scutoons\Domain\Admin\ListQuery;
use Scutoons\Domain\Admin\Segments;
use Scutoons\Domain\Admin\UserAdmin;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Campaigns\CampaignStore;
use Scutoons\Domain\Campaigns\Invites;
use Scutoons\Domain\Codes\CodeService;
use Scutoons\Domain\Inventory\InventoryService;
use Scutoons\Tests\TestCase;

/**
 * Segments, the account filters and promo batches (agents/research/GIFT-INVITES.md 1b; owner 2026-10-14).
 */
final class SegmentsTest extends TestCase
{
    public function testSegmentsByHandAndByFilter(): void
    {
        $segments = $this->get(Segments::class);
        $admin = $this->user('admin@example.com');
        $this->assertApiError(400, 'invalid_name', fn() => $segments->create($admin, ' '));
        $vip = $segments->create($admin, 'VIP');
        $this->assertApiError(409, 'name_taken', fn() => $segments->create($admin, 'VIP'));
        $anna = $this->user('anna@example.com');
        $ben = $this->user('ben@example.com');
        self::assertSame(1, $segments->add($admin, $vip['id'], ['ids' => [$anna['id'], $anna['id']]])['added']);
        // "all matching": every account the filters find, on every page
        $added = $segments->add($admin, $vip['id'], ['matching' => true, 'q' => 'ben@']);
        self::assertSame([1, 2], [$added['added'], $added['segment']['members']]);
        self::assertSame(1, $segments->remove($vip['id'], ['ids' => [$ben['id']]])['removed']);

        $list = $this->get(UserAdmin::class)->list(ListQuery::from(['segment' => $vip['id']], UserAdmin::SORTABLE, 'email', UserAdmin::FILTERS));
        self::assertSame(['anna@example.com'], array_column($list['items'], 'email'));
        self::assertSame([['id' => $vip['id'], 'name' => 'VIP']], $list['items'][0]['segments']);
        $none = $this->get(UserAdmin::class)->list(ListQuery::from(['segment' => 'none', 'q' => 'example.com'], UserAdmin::SORTABLE, 'email', UserAdmin::FILTERS));
        self::assertSame(['admin@example.com', 'ben@example.com'], array_column($none['items'], 'email'));

        // a campaign lets the segment invite; the segment can't go while it does
        $store = $this->get(CampaignStore::class);
        $c = $store->create($admin, ['name' => 'VIP invites', 'starts_at' => $this->clock->utc(-60), 'ends_at' => $this->clock->utc(86400)])['id'];
        $store->setStatus($c, 'started');
        $this->assertApiError(404, 'unknown_segment', fn() => $store->setAccounts($c, ['segment_id' => 'x']));
        $page = $store->setAccounts($c, ['segment_id' => $vip['id']]);
        self::assertSame(['segment', 1], [$page['mode'], $page['chosen']]);
        self::assertNotNull($store->offerFor($anna));
        self::assertNull($store->offerFor($ben));
        $this->assertApiError(409, 'segment_mode', fn() => $store->setAccounts($c, ['add' => [$ben['id']]]));
        $this->assertApiError(409, 'in_use', fn() => $segments->delete($vip['id']));
        // back to chosen accounts: the segment's members are the list
        $page = $store->setAccounts($c, ['all' => false]);
        self::assertSame(['chosen', 1], [$page['mode'], $page['chosen']]);
        $segments->delete($vip['id']);
    }

    public function testAccountFiltersBySignupEditionBatchAndCampaign(): void
    {
        $admin = $this->user('admin@example.com');
        // a product code of an edition, a promo batch, a gift invite
        $vinyl = $this->user('vinyl@example.com');
        $code = $this->code('membership', ['edition_id' => '3_catpope_vinyl', 'edition_number' => 4]);
        $this->get(CodeService::class)->claim($vinyl, $code, null, null);

        $batch = $this->get(Batches::class)->createPromo('Comic Con Wien 2026', 2, null, null);
        self::assertSame(['promo', null], [$batch['type'], $batch['editionId']]);
        $promo = $this->get(CodeAdmin::class)->create(['redirect_to' => 'https://scutoons.com/', 'type' => 'promo', 'code' => $batch['codes'][0]['code'], 'batch' => $batch['filename']]);
        $this->assertApiError(400, 'invalid_batch', fn() => $this->get(CodeAdmin::class)->create(['redirect_to' => 'https://scutoons.com/', 'type' => 'promo', 'batch' => '../x']));
        $fan = $this->user('fan@example.com');
        $this->db->run('UPDATE users SET signup_code = ? WHERE id = ?', [$promo['code'], $fan['id']]);

        $store = $this->get(CampaignStore::class);
        $c = $store->create($admin, ['name' => 'Invites', 'starts_at' => $this->clock->utc(-60), 'ends_at' => $this->clock->utc(30 * 86400)])['id'];
        $store->setAccounts($c, ['all' => true]);
        $store->setStatus($c, 'started');
        $this->get(InventoryService::class)->add((string) $vinyl['id'], 'cucumber', null, 1, 'story');
        $invite = $this->get(Invites::class)->create($vinyl, ['item_id' => 'cucumber', 'to' => 'friend@example.com']);
        $friend = $this->user('friend@example.com');
        $this->db->run('UPDATE users SET signup_code = ? WHERE id = ?', [$this->db->value('SELECT invite_code FROM campaign_invites WHERE id = ?', [$invite['id']]), $friend['id']]);
        $this->db->run("UPDATE campaign_invites SET status = 'joined', joined_user_id = ? WHERE id = ?", [$friend['id'], $invite['id']]);

        $emails = fn(array $filters) => $this->emails($filters);
        self::assertSame(['vinyl@example.com'], $emails(['edition' => '3_catpope_vinyl']));
        self::assertSame(['fan@example.com'], $emails(['batch' => $batch['filename']]));
        self::assertSame(['fan@example.com'], $emails(['signup' => 'promo']), 'an invite code is promo too, but counts as invite');
        self::assertSame(['friend@example.com'], $emails(['signup' => 'invite']));
        self::assertSame(['vinyl@example.com'], $emails(['campaign' => $c, 'campaign_role' => 'invited']));
        self::assertSame(['friend@example.com'], $emails(['campaign' => $c, 'campaign_role' => 'joined']));
        self::assertSame(['friend@example.com', 'vinyl@example.com'], $emails(['campaign' => $c]));

        // the campaign's "all matching": ticks every account the filters find
        $store->setAccounts($c, ['all' => false, 'remove' => [$admin['id']]]);
        $this->db->run('DELETE FROM campaign_inviters WHERE campaign_id = ?', [$c]);
        $page = $store->setAccounts($c, ['add_matching' => true, 'filters' => ['edition' => '3_catpope_vinyl']]);
        self::assertSame(['chosen', 1], [$page['mode'], $page['chosen']]);
        $filtered = $store->accounts($c, '', 1, 25, ['batch' => $batch['filename']]);
        self::assertSame([1, false], [$filtered['total'], $filtered['items'][0]['chosen']]);
        // the Codes list finds the promo batch
        $codes = $this->get(CodeAdmin::class)->list(ListQuery::from(['batch' => $batch['filename']], CodeAdmin::SORTABLE, 'code', CodeAdmin::FILTERS));
        self::assertSame([$promo['code']], array_column($codes['items'], 'code'));
    }

    /**
     * @param array<string, string> $filters
     * @return list<string> the matching addresses, sorted
     */
    private function emails(array $filters): array
    {
        [$where, $params] = AccountFilters::where('', $filters);
        return array_map('strval', array_column($this->db->all("SELECT u.email FROM users u $where ORDER BY u.email", $params), 'email'));
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
