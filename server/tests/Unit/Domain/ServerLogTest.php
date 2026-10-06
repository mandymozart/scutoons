<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use PHPUnit\Framework\TestCase;
use Scutoons\Domain\Admin\ServerLog;

/** The admin's "Server errors" (BACKLOG 10.11): PHP error log entries, newest first, messages only */
final class ServerLogTest extends TestCase
{
    private const LOG = "[06-Oct-2026 23:49:01 Europe/Berlin] PHP Fatal error:  Uncaught Error: No entry or class found for Transport in /web/api/src/Kernel.php:51\n"
        . "Stack trace:\n#0 /web/api/public/index.php(12): x()\n#1 {main}\n"
        . "[07-Oct-2026 00:01:02 Europe/Berlin] PHP Warning:  mail to someone@example.com failed, token=abcdef0123456789abcdef0123456789abcd\n"
        . "[07-Oct-2026 00:02:03 Europe/Berlin] [qr-api] RuntimeException: mysql://scutoons:s3cret@localhost/db down\n";

    public function testEntriesNewestFirstWithTheirStackTraceAndLevels(): void
    {
        $file = tempnam(sys_get_temp_dir(), 'log');
        file_put_contents($file, self::LOG);
        $log = ServerLog::read($file, null, '', 10, '/web');
        unlink($file);
        self::assertTrue($log['exists']);
        self::assertSame(['2026-10-07', '2026-10-06'], $log['days']);
        self::assertSame(['error', 'warning', 'fatal'], array_column($log['entries'], 'level'));
        self::assertSame('2026-10-06T23:49:01', $log['entries'][2]['at']);
        self::assertSame('Uncaught Error: No entry or class found for Transport in api/src/Kernel.php:51', $log['entries'][2]['message']);
        self::assertStringContainsString('#0 api/public/index.php(12)', $log['entries'][2]['detail']);
    }

    public function testNothingPersonalOrSecretIsShown(): void
    {
        $entries = ServerLog::parse(self::LOG);
        self::assertSame('mail to [email] failed, token=[hidden]', $entries[1]['message']);
        self::assertSame('[qr-api] RuntimeException: mysql://scutoons:[hidden]@localhost/db down', $entries[2]['message']);
        self::assertSame('Authorization: [hidden]', ServerLog::clean('Authorization: Bearer abc'));
        self::assertSame('key [token] here', ServerLog::clean('key 0123456789abcdef0123456789abcdef01 here'));
        self::assertSame('ScutoonsHttpControllersAdminServerLogController', ServerLog::clean('ScutoonsHttpControllersAdminServerLogController'), 'a long name without digits stays');
    }

    public function testFiltersByDayAndTextAndAMissingFile(): void
    {
        $file = tempnam(sys_get_temp_dir(), 'log');
        file_put_contents($file, self::LOG);
        self::assertCount(1, ServerLog::read($file, '2026-10-06')['entries']);
        self::assertCount(1, ServerLog::read($file, null, 'MYSQL')['entries']);
        self::assertCount(1, ServerLog::read($file, null, '', 1)['entries']);
        unlink($file);
        self::assertFalse(ServerLog::read($file)['exists']);
        self::assertFalse(ServerLog::read('')['exists']);
    }
}
