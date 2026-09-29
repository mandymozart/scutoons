<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Domain\Access\AccessTokens;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Codes\CodeRules;
use Scutoons\Domain\Codes\CodeService;
use Scutoons\Domain\Inventory\InventoryService;
use Scutoons\Tests\TestCase;

final class CodeServiceTest extends TestCase
{
    private CodeService $codes;

    protected function setUp(): void
    {
        parent::setUp();
        $this->codes = $this->get(CodeService::class);
    }

    public function testNormalizeAcceptsNanoidsOnly(): void
    {
        self::assertSame('Ab_-9', CodeRules::normalize(' Ab_-9 '));
        self::assertNull(CodeRules::normalize('a b'));
        self::assertNull(CodeRules::normalize(str_repeat('a', 33)));
        self::assertNull(CodeRules::normalize(['x']));
    }

    public function testExpiredAndRevoked(): void
    {
        self::assertTrue(CodeRules::isExpired(['expires_at' => '2000-01-01 00:00:00'], self::NOW));
        self::assertFalse(CodeRules::isExpired(['expires_at' => null], self::NOW));
        self::assertTrue(CodeRules::isRevoked(['revoked_at' => '2026-01-01 00:00:00']));
        self::assertTrue(CodeRules::isClaimed(['claimed_by' => null, 'email' => 'old@form.test']));
    }

    public function testOptionsOfAnEditionCodeAndEdition(): void
    {
        $code = $this->code('membership', ['edition_id' => '1_catpope_dago', 'edition_number' => 7]);
        $row = (array) $this->codes->find($code);
        $options = $this->codes->options($row);
        self::assertSame(['dago_hoodie', 'dago_heavy_t'], array_column($options, 'item_id'));
        self::assertSame(['id' => '1_catpope_dago', 'name' => 'Cat Pope Dago', 'number' => 7, 'count' => 37], $this->codes->edition($row));
    }

    public function testClaimSingleOptionIsAutomaticAndIdempotent(): void
    {
        $user = $this->user();
        $code = $this->code('membership', ['edition_id' => '3_catpope_vinyl', 'edition_number' => 12]);
        self::assertTrue($this->codes->claim($user, $code, null, null));
        self::assertFalse($this->codes->claim($user, $code, null, null), 'same owner again → already yours');
        $entries = $this->get(InventoryService::class)->entries((string) $user['id']);
        $vinyl = array_values(array_filter($entries, fn($e) => $e['item_id'] === 'awakening_vinyl'));
        self::assertCount(1, $vinyl);
        self::assertSame(12, $vinyl[0]['edition_number']);
        self::assertSame($code, $vinyl[0]['code']);
        self::assertSame(1, (int) $this->db->value("SELECT COUNT(*) FROM code_events WHERE code = ? AND event = 'claim'", [$code]));
    }

    public function testClaimByAnotherAccountIsRefused(): void
    {
        $code = $this->code('membership', ['edition_id' => '3_catpope_vinyl']);
        $this->codes->claim($this->user(), $code, null, null);
        $this->assertApiError(409, 'already_claimed', fn() => $this->codes->claim($this->user(), $code, null, null));
        self::assertSame(1, (int) $this->db->value("SELECT COUNT(*) FROM code_events WHERE code = ? AND event = 'claim_refused'", [$code]));
    }

    public function testChoiceIsRequiredAndValidated(): void
    {
        $user = $this->user();
        $code = $this->code('membership', ['edition_id' => '1_catpope_dago']);
        $error = $this->assertApiError(422, 'choose_item', fn() => $this->codes->claim($user, $code, null, null));
        self::assertCount(2, $error->extra['options']);
        $this->assertApiError(422, 'invalid_item', fn() => $this->codes->claim($user, $code, 'dago_hoodie', 'pink'));
        self::assertTrue($this->codes->claim($user, $code, 'dago_hoodie', 'black'));
    }

    public function testLegacyEmailClaimNeedsThatEmail(): void
    {
        $code = $this->code('membership', ['email' => 'buyer@example.com']);
        $this->assertApiError(409, 'email_required', fn() => $this->codes->claim($this->user(), $code, null, null));
        self::assertTrue($this->codes->claim($this->user('buyer@example.com'), $code, null, null));
    }

    public function testPromoRevokedAndExpiredCodes(): void
    {
        $user = $this->user();
        $this->assertApiError(422, 'not_claimable', fn() => $this->codes->claim($user, $this->code('promo'), null, null));
        $this->assertApiError(410, 'revoked', fn() => $this->codes->claim($user, $this->code('membership', ['revoked_at' => '2026-01-01 00:00:00']), null, null));
        $this->assertApiError(410, 'expired', fn() => $this->codes->claim($user, $this->code('membership', ['expires_at' => '2020-01-01 00:00:00']), null, null));
        $this->assertApiError(404, 'not_found', fn() => $this->codes->claim($user, 'zzzzzzzz', null, null));
    }

    public function testSignupRefusal(): void
    {
        self::assertNull($this->codes->signupRefusal($this->code('promo'), 'a@example.com'));
        self::assertNull($this->codes->signupRefusal($this->code('membership'), 'a@example.com'));
        self::assertSame('no_signup', $this->codes->signupRefusal($this->code('page'), 'a@example.com'));
        self::assertSame('not_found', $this->codes->signupRefusal(null, 'a@example.com'));
        $owned = $this->code('membership', ['edition_id' => '3_catpope_vinyl']);
        $this->codes->claim($this->user(), $owned, null, null);
        self::assertSame('already_claimed', $this->codes->signupRefusal($owned, 'a@example.com'));
    }

    public function testDeviceAccessForAnyHolderWithToken(): void
    {
        $code = $this->code('membership', ['edition_id' => '3_catpope_vinyl']);
        $this->codes->claim($this->user(), $code, null, null);
        $access = $this->codes->access($code, null, 'catpope');
        self::assertSame(['catalog'], $access['permissions']);
        self::assertTrue($access['claimed']);
        self::assertArrayNotHasKey('claimed_by_me', $access);
        $payload = $this->get(AccessTokens::class)->verify($access['access_token']);
        self::assertNotNull($payload);
        self::assertStringStartsWith('code:', $payload['sub']);
        self::assertStringNotContainsString($code, $payload['sub']);
        $this->codes->access($code, null, null);
        self::assertSame(1, (int) $this->db->value("SELECT COUNT(*) FROM code_events WHERE code = ? AND event = 'access'", [$code]), 'one access event per device/IP and day');
        $page = $this->codes->access($this->code('page'), null, null);
        self::assertSame([], $page['permissions']);
        self::assertArrayNotHasKey('access_token', $page);
    }

    public function testInfoNeverContainsAnEmail(): void
    {
        $code = $this->code('membership', ['email' => 'secret@example.com', 'edition_id' => '2_catpope_beetles']);
        $info = $this->codes->info($code, null);
        self::assertStringNotContainsString('secret@', (string) json_encode($info));
        self::assertTrue($info['claimed']);
        self::assertSame('2_catpope_beetles', $info['edition']['id']);
    }

    /** @param callable(): mixed $call */
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
