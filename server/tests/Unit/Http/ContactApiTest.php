<?php
declare(strict_types=1);

namespace Scutoons\Tests\Http;

use Psr\Http\Message\ResponseInterface;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Kernel;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

/** POST /api/contact – the public contact form (Domain\Contact\ContactForm) */
final class ContactApiTest extends TestCase
{
    /** @param array<string, mixed> $body */
    private function send(array $body, string $ip = '127.0.0.1'): ResponseInterface
    {
        $request = (new ServerRequestFactory())->createServerRequest('POST', 'http://localhost/api/contact', ['REMOTE_ADDR' => $ip])
            ->withHeader('Content-Type', 'application/json');
        $request->getBody()->write((string) json_encode($body));
        $request->getBody()->rewind();
        $container = Kernel::container(Settings::fixed(self::root(), $this->settings()));
        $container->set(Clock::class, $this->clock);
        return Kernel::app($container)->handle($request);
    }

    private function mails(): string
    {
        $path = $this->dir . '/mail.log';
        return is_file($path) ? (string) file_get_contents($path) : '';
    }

    /**
     * @param array<string, mixed> $over
     * @return array<string, mixed>
     */
    private static function message(array $over = []): array
    {
        return $over + ['topic' => 'code', 'email' => 'reader@example.com', 'name' => 'Ana', 'code' => 'ABC123', 'message' => "Der Code geht nicht.\nWas tun?", 'locale' => 'de', 'page' => '/verify', 'website' => '', 'elapsed' => 8000];
    }

    public function testAMessageGoesToTheMailboxWithTheSenderAsReplyTo(): void
    {
        $response = $this->send(self::message());
        self::assertSame(200, $response->getStatusCode(), (string) $response->getBody());
        $mail = $this->mails();
        self::assertStringContainsString('To: hello@scutoons.com', $mail);
        self::assertStringContainsString('Reply-To: <reader@example.com>', $mail);
        self::assertStringContainsString('[Kontakt · QR-Code / Code] Der Code geht nicht. Was tun?', $mail);
        self::assertStringContainsString("Code: ABC123", $mail);
        self::assertStringContainsString("Konto: nicht angemeldet", $mail);
        self::assertSame(1, (int) $this->db->value("SELECT COUNT(*) FROM mail_log WHERE template = 'contact' AND status = 'sent'"));
    }

    public function testSpamIsDroppedSilently(): void
    {
        self::assertSame(200, $this->send(self::message(['website' => 'https://spam.test']))->getStatusCode(), 'honeypot');
        self::assertSame(200, $this->send(self::message(['elapsed' => 800]))->getStatusCode(), 'too fast');
        self::assertSame('', $this->mails());
    }

    public function testInvalidInputIsRefused(): void
    {
        self::assertSame(400, $this->send(self::message(['email' => 'nope']))->getStatusCode());
        self::assertSame(400, $this->send(self::message(['message' => ' ']))->getStatusCode());
        self::assertSame(400, $this->send(self::message(['topic' => 'lottery']))->getStatusCode());
        self::assertSame('', $this->mails());
    }

    public function testFiveMessagesPerIpAndHour(): void
    {
        for ($i = 0; $i < 5; $i++) self::assertSame(200, $this->send(self::message(), '10.0.0.9')->getStatusCode());
        self::assertSame(429, $this->send(self::message(), '10.0.0.9')->getStatusCode());
        self::assertSame(200, $this->send(self::message(), '10.0.0.10')->getStatusCode(), 'another IP');
    }
}
