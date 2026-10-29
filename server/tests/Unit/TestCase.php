<?php
declare(strict_types=1);

namespace Scutoons\Tests;

use DI\Container;
use PHPUnit\Framework\TestCase as BaseTestCase;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Clock\FrozenClock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Kernel;
use Scutoons\Support\Ids;

/**
 * Unit tests of the domain services: the MariaDB test database (TEST_DB_*, phpunit.xml.dist – the container of
 * `npm run dev:db`, or CI's), emptied and migrated once per run, every test starting from that state again
 * (MysqlTestDatabase::reset); a frozen clock, isolated settings (no config.local.php, no QR_* environment), mails into
 * a temp log, files into a temp dir.
 */
abstract class TestCase extends BaseTestCase
{
    public const NOW = 1790000000; // 2026-09-21 (UTC)

    protected Container $container;
    protected Database $db;
    protected FrozenClock $clock;
    protected string $dir;

    protected function setUp(): void
    {
        // a limit some code sets for its request (PushSender: 60 s, wall time on Windows) must not cut the later tests
        set_time_limit(0);
        $this->dir = str_replace('\\', '/', sys_get_temp_dir()) . '/scutoons-unit-' . bin2hex(random_bytes(6));
        mkdir($this->dir . '/files', 0777, true);
        MysqlTestDatabase::reset(self::root());
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
        $mysql = MysqlTestDatabase::config();
        $database = [
            'DB_HOST' => $mysql['host'], 'DB_PORT' => $mysql['port'], 'DB_NAME' => $mysql['name'], 'DB_USER' => $mysql['user'], 'DB_PASS' => $mysql['pass'],
        ] + ($mysql['sql_mode'] !== null ? ['DB_SQL_MODE' => $mysql['sql_mode']] : []);
        return $database + [
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

    // ── Fixtures ──────────────────────────────────────────────────────────────────────────────────────────

    /**
     * A code in the qr table
     * @param array<string, mixed> $fields
     */
    protected function code(string $type = 'membership', array $fields = []): string
    {
        $code = $fields['code'] ?? 'c' . substr(bin2hex(random_bytes(4)), 0, 7);
        $this->db->run('INSERT INTO qr (id, code, redirect_to, type, email, created_at, claimed_at, expires_at, edition_id, edition_number, revoked_at, signature) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)', [
            Ids::uuid(), $code, $fields['redirect_to'] ?? 'https://scutoons.com/catpope', $type, $fields['email'] ?? '', $fields['created_at'] ?? $this->clock->local(), $this->clock->local(),
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

    /**
     * Stores a whole collection in its old one-document shape (`characters` with its list, `map` with locations / routes)
     * as its documents – the anchor + one `<kind>:<id>` per entry (agents/MAP-DOCUMENTS.md), like an import
     * @param array<string, mixed> $composed
     * @return int the anchor's version id
     */
    protected function storeCollection(string $anchor, array $composed, string $status = 'published', string $createdAt = '2026-09-30 00:00:00'): int
    {
        $spec = \Scutoons\Domain\Content\CollectionDocs::ofAnchor($anchor) ?? throw new \LogicException("$anchor is no collection");
        $first = 0;
        foreach (\Scutoons\Domain\Content\CollectionDocs::split($spec, json_decode((string) json_encode($composed))) as $key => $doc) {
            $kind = explode(':', $key)[0];
            $this->db->run('INSERT INTO entity_versions (entity_key, kind, toon, locale, json, author, created_at, status, message) VALUES (?, ?, NULL, NULL, ?, NULL, ?, ?, NULL)',
                [$key, $kind, (string) json_encode($doc, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_PRESERVE_ZERO_FRACTION), $createdAt, $status]);
            $first = $first ?: $this->db->lastInsertId();
        }
        return $first;
    }

    /**
     * The smallest chapter that fits its type (PUT /api/entities refuses others, ContentSchemas), as a JSON string
     * @param array<string, mixed> $fields
     */
    protected static function chapterJson(array $fields = []): string
    {
        return (string) json_encode($fields + ['title' => 'Eins', 'author' => 'A', 'number' => 1, 'language' => 'de', 'release' => '2026-01-01',
            'description' => '', 'thumbnail' => 'images/t.webp', 'groups' => []], JSON_UNESCAPED_SLASHES);
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
