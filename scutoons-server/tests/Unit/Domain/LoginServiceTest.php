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
        self::assertSame(['awakening_vinyl', 'camera'], $items, 'starter camera + the claimed vinyl');
        self::assertSame(['claim', 'login', 'signup'], array_column($this->db->all('SELECT event FROM code_events WHERE code = ? ORDER BY event', [$code]), 'event'));

        $this->expectApiError(410, 'already_used', fn() => $this->login->verify(['email' => 'new@example.com', 'otp' => $this->otp()]));
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
