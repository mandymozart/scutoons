<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Auth\LoginService;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Domain\RequestContext;
use Scutoons\Tests\TestCase;

final class LoginServiceTest extends TestCase
{
    private LoginService $login;

    protected function setUp(): void
    {
        parent::setUp();
        $this->login = $this->get(LoginService::class);
        $this->get(RequestContext::class)->set('203.0.113.9', 'device-unit-test', 'http://viewer.test');
    }

    public function testSignupWithAProductCodeCreatesTheAccountAndClaims(): void
    {
        $code = $this->code('membership', ['edition_id' => '3_catpope_vinyl', 'edition_number' => 3]);
        $sent = $this->login->request(['email' => ' New@Example.com ', 'code' => $code, 'locale' => 'de-AT', 'marketing_opt_in' => true]);
        self::assertSame('sent', $sent['status']);
        $mail = $this->lastMail();
        self::assertStringContainsString('To: new@example.com', $mail);
        self::assertStringContainsString('http://viewer.test/?login=', $mail, 'magic link to the calling viewer (allowed origin)');
        self::assertStringContainsString("Damit wird dein SCUTOONS-Konto erstellt.", $mail);

        $result = $this->login->verify(['email' => 'new@example.com', 'otp' => $this->otp()]);
        self::assertTrue($result['created']);
        self::assertSame([$code], $result['claimed']);
        self::assertSame(['member'], $result['user']['roles']);
        self::assertTrue($result['user']['marketing_opt_in']);
        self::assertNotNull($this->get(Sessions::class)->user($result['session_token']));
        $items = array_column($this->db->all('SELECT item_id FROM inventory WHERE user_id = ?', [$result['user']['id']]), 'item_id');
        sort($items);
        self::assertSame(['awakening_vinyl', 'camera', 'scutoons_sandals', 'scutoons_shorts', 'scutoons_swim_pants', 'scutoons_tshirt'], $items, 'starter camera + default wearables + the claimed vinyl');
        self::assertSame(['claim', 'login', 'signup'], array_column($this->db->all('SELECT event FROM code_events WHERE code = ? ORDER BY event', [$code]), 'event'));

        $this->expectApiError(410, 'already_used', fn() => $this->login->verify(['email' => 'new@example.com', 'otp' => $this->otp()]));
    }

    /** "This is mine" (agents/ITEMS.md "Ownership, claims, gifting"): an account to file a claim, the code stays where it is */
    public function testDisputeSignupWithSomeoneElsesCode(): void
    {
        $code = $this->code('membership', ['edition_id' => '3_catpope_vinyl']);
        $owner = $this->user('owner@example.com');
        $this->get(\Scutoons\Domain\Codes\CodeService::class)->claim($owner, $code, null, null);

        // without dispute: refused as before (no one-time code mailed)
        $this->login->request(['email' => 'buyer@example.com', 'code' => $code]);
        self::assertStringContainsString('There is no account for it yet', $this->lastMail());
        $this->clock->advance(1); // verify takes the newest request (same frozen millisecond = random order)
        $this->login->request(['email' => 'buyer@example.com', 'code' => $code, 'dispute' => true]);
        $mail = $this->lastMail();
        self::assertStringContainsString('This creates your SCUTOONS account.', $mail);
        self::assertStringNotContainsString('added to your inventory', $mail, 'nothing is claimed');
        $result = $this->login->verify(['email' => 'buyer@example.com', 'otp' => $this->otp()]);
        self::assertSame([true, true], [$result['created'], $result['dispute']]);
        self::assertArrayNotHasKey('claimed', $result);
        self::assertSame((string) $owner['id'], $this->db->value('SELECT claimed_by FROM qr WHERE code = ?', [$code]), 'the code stays with its owner');
        self::assertSame($code, $this->db->value('SELECT signup_code FROM users WHERE email = ?', ['buyer@example.com']));

        // dispute does not lift the other refusals
        $revoked = $this->code('membership', ['revoked_at' => '2026-01-01 00:00:00']);
        $this->login->request(['email' => 'other@example.com', 'code' => $revoked, 'dispute' => true]);
        self::assertStringContainsString('There is no account for it yet', $this->lastMail());
    }

    public function testNoAccountAndNoSignupCodeGetsAnExplanationOnly(): void
    {
        $sent = $this->login->request(['email' => 'nobody@example.com', 'locale' => 'en']);
        self::assertSame('sent', $sent['status'], 'no user enumeration');
        $mail = $this->lastMail();
        self::assertStringContainsString('There is no account for it yet', $mail);
        self::assertDoesNotMatchRegularExpression('/^\s{4}\d{6}\s*$/m', $mail, 'no one-time code');
        $this->expectApiError(400, 'invalid_otp', fn() => $this->login->verify(['email' => 'nobody@example.com', 'otp' => '123456']));
    }

    /** owner 2026-10-03: an address the old system registered codes to signs up without typing one – the oldest is claimed */
    public function testOldEmailRegistrationSignsUpWithoutACode(): void
    {
        $first = $this->code('membership', ['email' => 'Old@Example.com', 'created_at' => '2025-09-23 22:20:18']);
        $second = $this->code('membership', ['email' => 'old@example.com', 'created_at' => '2025-09-23 22:20:21']);
        $this->login->request(['email' => 'old@example.com', 'locale' => 'en']);
        self::assertStringNotContainsString('There is no account for it yet', $this->lastMail());

        $result = $this->login->verify(['email' => 'old@example.com', 'otp' => $this->otp()]);
        self::assertTrue($result['created']);
        self::assertSame([$first], $result['claimed']);
        $owner = fn(string $code) => $this->db->value('SELECT claimed_by FROM qr WHERE code = ?', [$code]);
        self::assertSame($result['user']['id'], $owner($first));
        self::assertNull($owner($second), 'the other one stays registered to the address until it is scanned');
    }

    public function testWrongCodesCountAndLockTheRequest(): void
    {
        $this->user('known@example.com');
        $this->login->request(['email' => 'known@example.com']);
        for ($i = 0; $i < 5; $i++) {
            $this->expectApiError(400, 'invalid_otp', fn() => $this->login->verify(['email' => 'known@example.com', 'otp' => $this->otp() === '000000' ? '111111' : '000000']));
        }
        $this->expectApiError(429, 'too_many_attempts', fn() => $this->login->verify(['email' => 'known@example.com', 'otp' => $this->otp()]));
    }

    public function testExistingAccountReportsAClaimErrorInsteadOfFailing(): void
    {
        $code = $this->code('membership', ['edition_id' => '1_catpope_dago']);
        $this->user('owner@example.com');
        $this->login->request(['email' => 'owner@example.com', 'code' => $code]);
        $result = $this->login->verify(['email' => 'owner@example.com', 'otp' => $this->otp()]);
        self::assertFalse($result['created']);
        self::assertSame([], $result['claimed']);
        self::assertSame('choose_item', $result['claim_error']['error']);
        self::assertCount(2, $result['claim_error']['options']);
    }

    public function testRevokedSignupCodeRefusesTheNewAccount(): void
    {
        $code = $this->code('promo');
        $this->login->request(['email' => 'late@example.com', 'code' => $code]);
        $this->db->run('UPDATE qr SET revoked_at = ? WHERE code = ?', [$this->clock->local(), $code]);
        $this->expectApiError(403, 'signup_code_required', fn() => $this->login->verify(['email' => 'late@example.com', 'otp' => $this->otp()]));
        self::assertNull($this->db->value('SELECT 1 FROM users WHERE email = ?', ['late@example.com']));
    }

    public function testMagicLinkFromTheLandingPageAndHourlyLimit(): void
    {
        $this->get(RequestContext::class)->set('203.0.113.9', null, null);
        $this->user('link@example.com');
        $this->login->request(['email' => 'link@example.com', 'code' => 'abc']);
        $found = preg_match('#http://qr\.test/\?login=([A-Za-z0-9_-]+)&c=abc#', $this->lastMail(), $m);
        self::assertSame(1, $found, 'link to QR_URL with the code');
        self::assertFalse($this->login->verify(['token' => $m[1] ?? ''])['created']);
        for ($i = 0; $i < 4; $i++) $this->login->request(['email' => 'link@example.com']);
        $this->expectApiError(429, 'rate_limited', fn() => $this->login->request(['email' => 'link@example.com']));
    }

    private function lastMail(): string
    {
        $log = (string) file_get_contents($this->dir . '/mail.log');
        return (string) substr($log, (int) strrpos($log, '=== '));
    }

    private function otp(): string
    {
        return preg_match('/^\s{4}(\d{6})\s*$/m', $this->lastMail(), $m) === 1 ? $m[1] : '';
    }

    /** @param callable(): mixed $call */
    private function expectApiError(int $status, string $code, callable $call): void
    {
        try {
            $call();
            self::fail("expected ApiError $status $code");
        } catch (ApiError $error) {
            self::assertSame([$status, $code], [$error->status, $error->errorCode], $error->getMessage());
        }
    }
}
