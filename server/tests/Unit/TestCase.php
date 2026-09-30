<?php
declare(strict_types=1);

namespace Scutoons\Tests;

use DI\Container;
use PHPUnit\Framework\TestCase as BaseTestCase;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Clock\FrozenClock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Infrastructure\Database\MigrationRunner;
use Scutoons\Kernel;
use Scutoons\Support\Ids;

/**
 * Unit tests of the domain services: a fresh SQLite database per test (a copy of a migrated template), a frozen
 * clock, isolated settings (no config.local.php, no QR_* environment), mails into a temp log, files into a temp dir.
 */
abstract class TestCase extends BaseTestCase
{
    public const NOW = 1790000000; // 2026-09-21 (UTC)
    private static ?string $template = null;

    protected Container $container;
    protected Database $db;
    protected FrozenClock $clock;
    protected string $dir;

    protected function setUp(): void
    {
        $this->dir = str_replace('\\', '/', sys_get_temp_dir()) . '/scutoons-unit-' . bin2hex(random_bytes(6));
        mkdir($this->dir . '/files', 0777, true);
        copy(self::template(), $this->dir . '/test.sqlite');
        $this->container = Kernel::container(Settings::fixed(self::root(), $this->settings()));
        $this->clock = new FrozenClock(self::NOW);
        $this->container->set(Clock::class, $this->clock);
        $this->db = $this->container->get(Database::class);
    }

    protected function tearDown(): void
    {
        $this->db->close();
        unset($this->container, $this->db);
        gc_collect_cycles();
        self::remove($this->dir);
    }

    /** @return array<string, string> */
    protected function settings(): array
    {
        return [
            'DB_SQLITE_PATH' => $this->dir . '/test.sqlite',
            'AUTH_SECRET' => str_repeat('unit-test-secret-', 3),
            'MAIL_TRANSPORT' => 'log',
            'MAIL_LOG_PATH' => $this->dir . '/mail.log',
            'FILES_PATH' => $this->dir . '/files',
            'KEYPAIR_PATH' => $this->dir . '/keypair.json',
            'CACHE_DRIVER' => 'file',
            'CACHE_PATH' => $this->dir . '/cache',
            'LOG_PATH' => '',
            'APP_URL' => 'http://viewer.test',
            'QR_URL' => 'http://qr.test',
            'ALLOWED_ORIGINS' => 'http://viewer.test',
        ];
    }

    /**
     * @template T of object
     * @param class-string<T> $class
     * @return T
     */
    protected function get(string $class): object
    {
        return $this->container->get($class);
    }

    public static function root(): string
    {
        return str_replace('\\', '/', dirname(__DIR__, 2));
    }

    /** A migrated empty database, made once per test run */
    private static function template(): string
    {
        if (self::$template === null) {
            $file = str_replace('\\', '/', sys_get_temp_dir()) . '/scutoons-unit-template-' . getmypid() . '.sqlite';
            @unlink($file);
            (new MigrationRunner(Database::sqlite($file), self::root() . '/migrations'))->migrate(static fn() => null);
            register_shutdown_function(static fn() => @unlink($file));
            self::$template = $file;
        }
        return self::$template;
    }

    // ── Fixtures ──────────────────────────────────────────────────────────────────────────────────────────

    /**
     * A code in the qr table
     * @param array<string, mixed> $fields
     */
    protected function code(string $type = 'membership', array $fields = []): string
    {
        $code = $fields['code'] ?? 'c' . substr(bin2hex(random_bytes(4)), 0, 7);
        $this->db->run('INSERT INTO qr (id, code, redirect_to, type, email, created_at, claimed_at, expires_at, edition_id, edition_number, revoked_at, signature) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)', [
            Ids::uuid(), $code, $fields['redirect_to'] ?? 'https://scutoons.com/catpope', $type, $fields['email'] ?? '', $this->clock->local(), $this->clock->local(),
            $fields['expires_at'] ?? null, $fields['edition_id'] ?? null, $fields['edition_number'] ?? null, $fields['revoked_at'] ?? null, $fields['signature'] ?? null,
        ]);
        return (string) $code;
    }

    /** @return array<string, mixed> a users row */
    protected function user(string $email = ''): array
    {
        $email = $email !== '' ? $email : 'u' . bin2hex(random_bytes(4)) . '@example.com';
        return $this->db->transaction(fn() => $this->get(\Scutoons\Domain\Accounts\Users::class)->create($email, 'en'));
    }

    private static function remove(string $path): void
    {
        if (is_dir($path)) {
            foreach (scandir($path) ?: [] as $entry) {
                if ($entry !== '.' && $entry !== '..') self::remove("$path/$entry");
            }
            @rmdir($path);
        } elseif (is_file($path)) {
            @unlink($path);
        }
    }
}
