<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Auth\LoginService;
use Scutoons\Domain\Newsletter\Campaigns;
use Scutoons\Domain\Newsletter\Contacts;
use Scutoons\Domain\Newsletter\NewsletterRenderer;
use Scutoons\Domain\Newsletter\NewsletterSender;
use Scutoons\Domain\Newsletter\NewsletterSignin;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Mail\Transport;
use Scutoons\Tests\TestCase;

/** The `{signin}` button of a newsletter (NewsletterSignin, #65): one click = signed in, the text per recipient */
final class NewsletterSigninTest extends TestCase
{
    protected function settings(): array
    {
        return ['NEWSLETTER_DELAY_MS' => '0', 'API_URL' => 'http://api.test', 'APP_URL' => 'https://app.test'] + parent::settings();
    }

    /** @return array<string, string> recipient → the token of its `?signin=` link ('' = the mail has none) */
    private function send(): array
    {
        $transport = new FakeNewsletterTransport(Settings::fixed(self::root(), $this->settings()), []);
        $this->container->set(Transport::class, $transport);
        $id = (int) $this->get(Campaigns::class)->create([
            'title' => 'Sign in', 'audiences' => ['members_all', 'contacts'],
            'subject_de' => 'Hallo', 'subject_en' => 'Hi',
            'body_de' => "Kapitel 3 ist da.\n\n{signin}\n\nBis bald", 'body_en' => "Chapter 3 is out.\n\n{signin}",
        ], 'admin:x')['id'];
        $this->get(Campaigns::class)->start($id, 'admin:x');
        $this->get(NewsletterSender::class)->batch();
        $links = [];
        foreach ($transport->sent as $mail) {
            preg_match('#\?signin=([A-Za-z0-9_-]+)#', $mail['text'], $m);
            $links[$mail['to']] = $m[1] ?? '';
            $this->texts[$mail['to']] = $mail['text'];
        }
        return $links;
    }

    /** @var array<string, string> */
    private array $texts = [];

    /** @return array<string, mixed> */
    private function verify(string $token): array
    {
        return $this->get(LoginService::class)->verify(['token' => $token]);
    }

    public function testMembersSignInQrContactsSignUpOthersGetNoButton(): void
    {
        $this->user('member@example.com');
        $this->db->run('UPDATE users SET locale = ? WHERE email = ?', ['de', 'member@example.com']);
        $code = $this->code('membership', ['email' => 'qr@example.com']); // the old QR form: a code waits for this address
        $this->get(Contacts::class)->import("qr@example.com, en\nevent@example.com, de", 'qr-claim', null, null, false, 'admin:x');

        $links = $this->send();
        self::assertNotSame('', $links['member@example.com']);
        self::assertStringContainsString("Anmelden: https://app.test/?signin=", $this->texts['member@example.com']);
        self::assertStringContainsString("Create your account – your QR code is waiting: https://app.test/en/?signin=", $this->texts['qr@example.com']);
        self::assertSame('', $links['event@example.com']);
        self::assertStringContainsString("Kapitel 3 ist da.\n\nBis bald", $this->texts['event@example.com'], 'no signup code: the paragraph is left out');

        self::assertFalse($this->verify($links['member@example.com'])['created']);
        $signup = $this->verify($links['qr@example.com']);
        self::assertTrue($signup['created'], 'the QR contact gets its account …');
        self::assertSame([$code], $signup['claimed'], '… with its code');

        try {
            $this->verify($links['member@example.com']);
            self::fail('a link signs in once');
        } catch (ApiError $error) {
            self::assertSame('already_used', $error->errorCode);
        }
    }

    public function testStaffGetTheAccountPageAndTheLinksStayOutOfTheCodeLogin(): void
    {
        $admin = $this->user('boss@example.com');
        $this->get(Roles::class)->grant((string) $admin['id'], Roles::ADMIN, 'test');
        $this->user('reader@example.com');
        // a reader asks for a login code, then the newsletter goes out: the code still works
        $login = $this->get(LoginService::class);
        $login->request(['email' => 'reader@example.com']);
        $links = $this->send();
        self::assertSame('', $links['boss@example.com']);
        self::assertStringContainsString('Sign in: https://app.test/en/account', $this->texts['boss@example.com']);
        self::assertSame(0, (int) $this->db->value("SELECT COUNT(*) FROM login_requests WHERE email = 'boss@example.com' AND ip_hash = ?", [NewsletterSignin::IP_HASH]));

        $mail = $this->db->one("SELECT * FROM login_requests WHERE email = 'reader@example.com' AND ip_hash <> ?", [NewsletterSignin::IP_HASH]);
        self::assertNotNull($mail);
        $link = $this->db->one("SELECT created_at, expires_at FROM login_requests WHERE email = 'reader@example.com' AND ip_hash = ?", [NewsletterSignin::IP_HASH]);
        self::assertNotNull($link);
        $days = (strtotime((string) $link['expires_at']) - strtotime((string) $link['created_at'])) / 86400;
        self::assertEqualsWithDelta(14, $days, 0.1, 'valid NEWSLETTER_SIGNIN_DAYS');
        try {
            $login->verify(['email' => 'reader@example.com', 'otp' => '000000']);
        } catch (ApiError $error) {
            self::assertSame('invalid_otp', $error->errorCode);
        }
        self::assertSame(1, (int) $this->db->value('SELECT attempts FROM login_requests WHERE id = ?', [$mail['id']]), 'the code mail is still the newest request');
    }

    public function testThePreviewShowsAMembersButtonWithoutALoginRequest(): void
    {
        $mail = $this->get(NewsletterRenderer::class)->render(['subject' => 'S', 'body' => "A\n\n{signin}"], 'en', 'nobody@example.com', null);
        self::assertStringContainsString('href="https://app.test/en/account"', $mail['html']);
        self::assertSame(0, (int) $this->db->value('SELECT COUNT(*) FROM login_requests'));
    }
}
