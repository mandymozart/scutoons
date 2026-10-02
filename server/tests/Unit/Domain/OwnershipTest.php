<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use PDOException;
use Scutoons\Domain\Admin\ListQuery;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Avatar\AvatarService;
use Scutoons\Domain\Codes\CodeService;
use Scutoons\Domain\Inventory\InventoryService;
use Scutoons\Domain\Ownership\Gifts;
use Scutoons\Domain\Ownership\OwnershipClaims;
use Scutoons\Domain\Ownership\OwnershipTransfers;
use Scutoons\Tests\TestCase;

/**
 * Ownership, claims, gifting (agents/ITEMS.md "Ownership, claims, gifting"): the transfer core (DB owner guard,
 * inventory move, avatar slots), the claim lifecycle with evidence files and its mails, the admin transfer, gifts and
 * their rules, the "you received" notices.
 */
final class OwnershipTest extends TestCase
{
    private const PNG = "\x89PNG\r\n\x1A\n0000IHDR";

    /** @return array<string, string> */
    protected function settings(): array
    {
        return ['MAIL_ADMIN_TO' => 'boss@example.com'] + parent::settings();
    }

    // ── Transfer core ─────────────────────────────────────────────────────────────────────────────────────

    public function testTheDbGuardStillForbidsADirectOwnerChange(): void
    {
        [$a] = $this->owned();
        $b = $this->user();
        $code = (string) $this->db->value('SELECT code FROM qr WHERE claimed_by = ?', [$a['id']]);
        $this->expectException(PDOException::class);
        $this->db->run('UPDATE qr SET claimed_by = ? WHERE code = ?', [$b['id'], $code]);
    }

    public function testMoveCodeMovesEntriesAndClearsTheAvatarSlot(): void
    {
        [$a, $code] = $this->owned('1_catpope_dago', 'dago_hoodie', 'black');
        $entry = (string) $this->db->value('SELECT id FROM inventory WHERE code = ?', [$code]);
        $this->get(AvatarService::class)->update((string) $a['id'], ['slots' => ['torso' => $entry]]);
        self::assertSame($entry, $this->db->value('SELECT entry_id FROM avatar_slots WHERE user_id = ? AND slot = ?', [$a['id'], 'torso']));
        $b = $this->user();
        $admin = $this->user();

        $moved = $this->db->transaction(fn() => $this->get(OwnershipTransfers::class)->moveCode($code, $b, 'admin', (string) $admin['id'], 'support case'));
        self::assertSame([(string) $a['id'], (string) $b['id'], [$entry], 'dago_hoodie', 'black'], [$moved['from'], $moved['to'], $moved['entries'], $moved['item_id'], $moved['variant']]);
        $qr = (array) $this->db->one('SELECT claimed_by, email FROM qr WHERE code = ?', [$code]);
        self::assertSame([(string) $b['id'], $b['email']], [$qr['claimed_by'], $qr['email']]);
        $row = (array) $this->db->one('SELECT user_id, source, code, edition_number FROM inventory WHERE id = ?', [$entry]);
        self::assertSame([(string) $b['id'], 'qr', $code, 5], [$row['user_id'], $row['source'], $row['code'], (int) $row['edition_number']], 'same entry, edition number kept');
        self::assertNotSame($entry, $this->db->value('SELECT entry_id FROM avatar_slots WHERE user_id = ? AND slot = ?', [$a['id'], 'torso']), 'not worn by A any more');
        $history = $this->get(OwnershipTransfers::class)->history($code);
        self::assertSame(['admin', $a['email'], $b['email'], $admin['email'], 'support case'],
            [$history[0]['kind'], $history[0]['from']['email'], $history[0]['to']['email'], $history[0]['by']['email'], $history[0]['note']]);
        // the previous owner can't claim it back the normal way
        try {
            $this->get(CodeService::class)->claim($a, $code, null, null);
            self::fail('expected already_claimed');
        } catch (ApiError $error) {
            self::assertSame('already_claimed', $error->errorCode);
        }
        $this->assertApiError(409, 'already_owner', fn() => $this->db->transaction(fn() => $this->get(OwnershipTransfers::class)->moveCode($code, $b, 'admin', null, 'x')));
    }

    public function testAdminTransferOfAnUnregisteredCodeClaimsItAndMailsTheNewOwner(): void
    {
        $code = $this->code('membership', ['edition_id' => '3_catpope_vinyl', 'edition_number' => 9]);
        $b = $this->user('new@example.com');
        $admin = $this->user();
        $this->assertApiError(400, 'note_required', fn() => $this->get(OwnershipTransfers::class)->adminTransfer($admin, $code, ['to' => 'new@example.com']));
        $this->assertApiError(404, 'unknown_user', fn() => $this->get(OwnershipTransfers::class)->adminTransfer($admin, $code, ['to' => 'nobody@example.com', 'note' => 'x']));
        $moved = $this->get(OwnershipTransfers::class)->adminTransfer($admin, $code, ['to' => 'NEW@example.com', 'note' => 'bought at the show']);
        self::assertNull($moved['from']);
        self::assertSame('awakening_vinyl', $moved['item_id']);
        self::assertSame((string) $b['id'], $this->db->value('SELECT claimed_by FROM qr WHERE code = ?', [$code]));
        self::assertSame(['ownership_transferred_to'], $this->mails('new@example.com'));
        $this->assertApiError(422, 'not_claimable', fn() => $this->get(OwnershipTransfers::class)->adminTransfer($admin, $this->code('promo'), ['to' => $b['id'], 'note' => 'x']));
    }

    // ── Claims ────────────────────────────────────────────────────────────────────────────────────────────

    public function testClaimLifecycleWithFilesAndMails(): void
    {
        [$owner, $code] = $this->owned('3_catpope_vinyl', null, null, 'owner@example.com');
        $claimant = $this->user('claimant@example.com');
        $claims = $this->get(OwnershipClaims::class);

        $this->assertApiError(400, 'statement_required', fn() => $claims->file($claimant, ['code' => $code, 'statement' => 'mine']));
        $this->assertApiError(409, 'not_registered', fn() => $claims->file($claimant, ['code' => $this->code('membership'), 'statement' => 'I bought this at the show.']));
        $this->assertApiError(409, 'already_yours', fn() => $claims->file($owner, ['code' => $code, 'statement' => 'I bought this at the show.']));
        $this->assertApiError(404, 'not_found', fn() => $claims->file($claimant, ['code' => 'nope', 'statement' => 'I bought this at the show.']));

        $claim = $claims->file($claimant, ['code' => $code, 'statement' => 'I bought this at the Vienna show.']);
        self::assertSame(['open', $code, []], [$claim['status'], $claim['code'], $claim['files']]);
        self::assertArrayNotHasKey('owner', $claim, 'the claimant never learns who owns it');
        self::assertSame(['ownership_claim_received'], $this->mails('claimant@example.com'));
        self::assertSame(['ownership_claim_admin'], $this->mails('boss@example.com'));
        $error = $this->assertApiError(409, 'claim_open', fn() => $claims->file($claimant, ['code' => $code, 'statement' => 'Again, it is mine really.']));
        self::assertSame($claim['id'], $error->extra['claim']['id']);

        // evidence: real type by the first bytes, size and count limits, only the claimant
        $withFile = $claims->addFile($claimant, $claim['id'], self::PNG, 'receipt <1>.png');
        self::assertSame([['name' => 'receipt _1_.png', 'type' => 'image/png', 'size' => strlen(self::PNG)]],
            array_map(fn($f) => array_diff_key($f, ['id' => 1]), $withFile['files']));
        $this->assertApiError(415, 'unsupported_type', fn() => $claims->addFile($claimant, $claim['id'], '<html>', 'x.png'));
        $this->assertApiError(413, 'file_too_large', fn() => $claims->addFile($claimant, $claim['id'], "%PDF-" . str_repeat('x', 5242880), 'big.pdf'));
        $this->assertApiError(404, 'not_found', fn() => $claims->addFile($owner, $claim['id'], self::PNG, 'x.png'));
        [$bytes, $type] = $claims->adminFile($claim['id'], $withFile['files'][0]['id']);
        self::assertSame([self::PNG, 'image/png'], [$bytes, $type]);

        // admin: list (owner visible), detail, transfer
        $list = $claims->list(ListQuery::from(['status' => 'open'], OwnershipClaims::SORTABLE, '-created_at', OwnershipClaims::FILTERS));
        self::assertSame(1, $list['pagination']['total']);
        self::assertSame(['claimant@example.com', 'owner@example.com', 1], [$list['items'][0]['claimant']['email'], $list['items'][0]['owner']['email'], $list['items'][0]['files']]);
        self::assertSame(1, $claims->openCount());
        $detail = $claims->detail($claim['id']);
        self::assertSame('owner@example.com', $detail['current_owner']['email']);
        self::assertSame([false, 'keys_missing'], [$detail['certificate']['valid'], $detail['certificate']['reason']], 'the certificate check runs (no keypair in the unit tests → its reason)');

        $admin = $this->user('admin@example.com');
        $this->assertApiError(400, 'note_required', fn() => $claims->transfer($admin, $claim['id'], []));
        $after = $claims->transfer($admin, $claim['id'], ['note' => 'receipt matches']);
        self::assertSame(['transferred', 'admin@example.com', 'receipt matches'], [$after['claim']['status'], $after['claim']['admin']['email'], $after['claim']['admin_note']]);
        self::assertSame((string) $claimant['id'], $this->db->value('SELECT claimed_by FROM qr WHERE code = ?', [$code]));
        self::assertSame((string) $claimant['id'], $this->db->value('SELECT user_id FROM inventory WHERE code = ?', [$code]));
        self::assertSame('claim', $after['history'][0]['kind']);
        self::assertSame(['ownership_transferred_from'], $this->mails('owner@example.com'));
        self::assertSame(['ownership_claim_received', 'ownership_transferred_to'], $this->mails('claimant@example.com'));
        $this->assertApiError(409, 'claim_closed', fn() => $claims->reject($admin, $claim['id'], ['note' => 'x']));
        $this->assertApiError(409, 'claim_closed', fn() => $claims->addFile($claimant, $claim['id'], self::PNG, 'late.png'));
        self::assertSame(['transferred'], array_column($claims->mine((string) $claimant['id']), 'status'));
        // the claimant sees the move as a notice
        self::assertSame([['claim', $code, null]], array_map(fn($r) => [$r['kind'], $r['code'], $r['from']], $this->get(OwnershipTransfers::class)->unseen((string) $claimant['id'])));
    }

    public function testRejectAndWithdraw(): void
    {
        [, $code] = $this->owned();
        $claimant = $this->user('c2@example.com');
        $claims = $this->get(OwnershipClaims::class);
        $claim = $claims->file($claimant, ['code' => $code, 'statement' => 'It was a present for me.']);
        $rejected = $claims->reject($this->user(), $claim['id'], ['note' => 'no proof, owner has the certificate']);
        self::assertSame('rejected', $rejected['claim']['status']);
        self::assertSame(['ownership_claim_received', 'ownership_claim_rejected'], $this->mails('c2@example.com'));
        $again = $claims->file($claimant, ['code' => $code, 'statement' => 'Now with the receipt attached.']);
        self::assertSame('withdrawn', $claims->withdraw((string) $claimant['id'], $again['id'])['status']);
        $this->assertApiError(409, 'claim_closed', fn() => $claims->withdraw((string) $claimant['id'], $again['id']));
        $this->assertApiError(404, 'not_found', fn() => $claims->withdraw((string) $this->user()['id'], $again['id']));
    }

    public function testClaimsPerDayAreLimited(): void
    {
        $claimant = $this->user();
        $claims = $this->get(OwnershipClaims::class);
        for ($i = 0; $i < 5; $i++) $claims->file($claimant, ['code' => $this->owned()[1], 'statement' => 'This one is mine as well.']);
        $this->assertApiError(429, 'rate_limited', fn() => $claims->file($claimant, ['code' => $this->owned()[1], 'statement' => 'This one is mine as well.']));
    }

    // ── Gifts ─────────────────────────────────────────────────────────────────────────────────────────────

    public function testGiftAStackableItem(): void
    {
        $giver = $this->user('giver@example.com');
        $friend = $this->user('friend@example.com');
        $this->get(InventoryService::class)->add((string) $giver['id'], 'cucumber', null, 5, 'story');
        $gifts = $this->get(Gifts::class);

        $this->assertApiError(400, 'self_gift', fn() => $gifts->gift($giver, ['item_id' => 'cucumber', 'to' => 'Giver@example.com']));
        $this->assertApiError(400, 'invalid_email', fn() => $gifts->gift($giver, ['item_id' => 'cucumber', 'to' => 'nope']));
        $this->assertApiError(404, 'recipient_not_found', fn() => $gifts->gift($giver, ['item_id' => 'cucumber', 'to' => 'stranger@example.com']));
        $this->assertApiError(422, 'missing', fn() => $gifts->gift($giver, ['item_id' => 'cucumber', 'qty' => 6, 'to' => 'friend@example.com']));

        $gift = $gifts->gift($giver, ['item_id' => 'cucumber', 'qty' => 3, 'to' => 'friend@example.com']);
        self::assertSame(['cucumber', 3, null, 'friend@example.com'], [$gift['item_id'], $gift['qty'], $gift['code'], $gift['to']]);
        $inventory = $this->get(InventoryService::class);
        self::assertSame([2, 3], [$inventory->count((string) $giver['id'], 'cucumber'), $inventory->count((string) $friend['id'], 'cucumber')]);
        self::assertSame('gift', $this->db->value("SELECT source FROM inventory WHERE user_id = ? AND item_id = 'cucumber'", [$friend['id']]));
        self::assertSame(['gift_received'], $this->mails('friend@example.com'));

        // the notice: who sent what, once
        $transfers = $this->get(OwnershipTransfers::class);
        $notice = $transfers->unseen((string) $friend['id']);
        self::assertSame([['gift', 'cucumber', 3, 'giver@example.com']], array_map(fn($r) => [$r['kind'], $r['item_id'], $r['qty'], $r['from']], $notice));
        self::assertSame(1, $transfers->markSeen((string) $friend['id'], [$notice[0]['id']]));
        self::assertSame([], $transfers->unseen((string) $friend['id']));

        // re-gift cooldown: the friend can't pass the 3 on for 24 h, only own older ones
        $error = $this->assertApiError(409, 'cooldown', fn() => $gifts->gift($friend, ['item_id' => 'cucumber', 'to' => 'giver@example.com']));
        self::assertSame(86400, $error->extra['retry_after']);
        $this->clock->advance(86401);
        self::assertSame(1, $gifts->gift($friend, ['item_id' => 'cucumber', 'to' => 'giver@example.com'])['qty']);
    }

    public function testGiftRules(): void
    {
        $giver = $this->user();
        $friend = $this->user('friend2@example.com');
        $gifts = $this->get(Gifts::class);
        $inventory = $this->get(InventoryService::class);
        // starters / the basic dress are not giftable
        $this->assertApiError(403, 'not_giftable', fn() => $gifts->gift($giver, ['item_id' => 'camera', 'to' => 'friend2@example.com']));
        $this->assertApiError(403, 'not_giftable', fn() => $gifts->gift($giver, ['item_id' => 'scutoons_tshirt', 'to' => 'friend2@example.com']));
        $this->assertApiError(404, 'not_found', fn() => $gifts->gift($giver, ['item_id' => 'testo', 'to' => 'friend2@example.com']));
        // the recipient's collection limit
        $inventory->add((string) $friend['id'], 'tuna_can', null, 99, 'story');
        $inventory->add((string) $giver['id'], 'tuna_can', null, 2, 'story');
        $error = $this->assertApiError(409, 'max_reached', fn() => $gifts->gift($giver, ['item_id' => 'tuna_can', 'to' => 'friend2@example.com']));
        self::assertSame(99, $error->extra['max']);
        // non-stackable: one piece at a time
        $inventory->add((string) $giver['id'], 'cucumber_camera', null, 1, 'craft');
        $this->assertApiError(400, 'invalid_qty', fn() => $gifts->gift($giver, ['item_id' => 'cucumber_camera', 'qty' => 2, 'to' => 'friend2@example.com']));
        // daily limit: 10 gifts per 24 h
        $inventory->add((string) $giver['id'], 'weed', null, 20, 'story');
        for ($i = 0; $i < 10; $i++) $gifts->gift($giver, ['item_id' => 'weed', 'to' => 'friend2@example.com']);
        $this->assertApiError(429, 'daily_limit', fn() => $gifts->gift($giver, ['item_id' => 'weed', 'to' => 'friend2@example.com']));
    }

    public function testGiftingACodeBoundItemMovesTheCodeAndWornItemsMustComeOffFirst(): void
    {
        [$giver, $code] = $this->owned('1_catpope_dago', 'dago_heavy_t', 'white', 'g3@example.com');
        $friend = $this->user('friend3@example.com');
        $entry = (string) $this->db->value('SELECT id FROM inventory WHERE code = ?', [$code]);
        $gifts = $this->get(Gifts::class);
        $this->get(AvatarService::class)->update((string) $giver['id'], ['slots' => ['torso' => $entry]]);
        $this->assertApiError(409, 'worn', fn() => $gifts->gift($giver, ['entry_id' => $entry, 'to' => 'friend3@example.com']));
        $this->get(AvatarService::class)->update((string) $giver['id'], ['slots' => ['torso' => null]]);

        $gift = $gifts->gift($giver, ['entry_id' => $entry, 'to' => 'friend3@example.com']);
        self::assertSame([$code, 1], [$gift['code'], $gift['qty']]);
        self::assertSame((string) $friend['id'], $this->db->value('SELECT claimed_by FROM qr WHERE code = ?', [$code]));
        self::assertSame([(string) $friend['id'], 'gift'], array_values((array) $this->db->one('SELECT user_id, source FROM inventory WHERE id = ?', [$entry])));
        self::assertSame('gift', $this->get(OwnershipTransfers::class)->history($code)[0]['kind']);
        // the new owner must wait before passing it on
        $this->assertApiError(409, 'cooldown', fn() => $gifts->gift($friend, ['entry_id' => $entry, 'to' => 'g3@example.com']));
        $this->clock->advance(86401);
        self::assertSame($code, $gifts->gift($friend, ['entry_id' => $entry, 'to' => 'g3@example.com'])['code']);
    }

    /** Production runs the code before the migration is applied: GET /me/ and the read paths keep working, writes say 503 */
    public function testWorksBeforeTheMigration(): void
    {
        [$user, $code] = $this->owned();
        foreach (['item_transfers', 'ownership_claims', 'ownership_claim_files'] as $table) $this->db->exec("DROP TABLE $table");
        $me = $this->get(\Scutoons\Domain\Accounts\AccountService::class)->me($user);
        self::assertSame([], $me['received']);
        self::assertSame([], $this->get(OwnershipTransfers::class)->history($code));
        $claims = $this->get(OwnershipClaims::class);
        self::assertSame([[], 0, 0], [$claims->mine((string) $user['id']), $claims->openCount(),
            $claims->list(ListQuery::from([], OwnershipClaims::SORTABLE, '-created_at', OwnershipClaims::FILTERS))['pagination']['total']]);
        $this->assertApiError(503, 'not_available', fn() => $this->get(Gifts::class)->gift($user, ['item_id' => 'cucumber', 'to' => 'x@example.com']));
        $this->assertApiError(503, 'not_available', fn() => $claims->file($this->user(), ['code' => $code, 'statement' => 'It is mine, really.']));
    }

    // ── Helpers ───────────────────────────────────────────────────────────────────────────────────────────

    /**
     * A user owning a fresh code of an edition (claimed the normal way)
     * @return array{0: array<string, mixed>, 1: string}
     */
    private function owned(string $edition = '3_catpope_vinyl', ?string $item = null, ?string $variant = null, string $email = ''): array
    {
        $user = $this->user($email);
        $code = $this->code('membership', ['edition_id' => $edition, 'edition_number' => 5]);
        $this->get(CodeService::class)->claim($user, $code, $item, $variant);
        return [$user, $code];
    }

    /** @return list<string> the templates mailed to an address, oldest first */
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
