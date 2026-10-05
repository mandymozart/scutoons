<?php
declare(strict_types=1);

namespace Scutoons\Tests\Infrastructure;

use PHPUnit\Framework\TestCase;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Mail\Imap;

/** The Embassy inbox's server: its own host, else SMTP_HOST – only with an Embassy mailbox login (never no-reply's) */
final class ImapHostTest extends TestCase
{
    /** @param array<string, string> $values */
    private static function host(array $values): string
    {
        return Imap::host(Settings::fixed(dirname(__DIR__, 3), $values + ['SMTP_HOST' => 'w0.kasserver.com']));
    }

    public function testFallsBackToTheSmtpHostOnlyWithAnEmbassyLogin(): void
    {
        self::assertSame('imap.example.com', self::host(['EMBASSY_IMAP_HOST' => 'imap.example.com']));
        self::assertSame('w0.kasserver.com', self::host(['EMBASSY_SMTP_USER' => 'm0hello']));
        self::assertSame('w0.kasserver.com', self::host(['EMBASSY_IMAP_USER' => 'm0hello']));
        self::assertSame('', self::host(['SMTP_USER' => 'm0noreply']), 'the no-reply login alone never turns the sync on');
    }
}
