<?php
declare(strict_types=1);

namespace Scutoons\Tests\Infrastructure;

use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Infrastructure\Database\MigrationRunner;
use Scutoons\Infrastructure\Storage\FileStore;
use Scutoons\Support\Ids;
use Scutoons\Tests\TestCase;

final class InfrastructureTest extends TestCase
{
    public function testSettingsDefaultsListsAndPaths(): void
    {
        $settings = Settings::fixed(self::root(), ['ALLOWED_ORIGINS' => ' a, ,b ', 'FILES_PATH' => 'storage/x', 'KEYPAIR_PATH' => 'C:/abs/k.json']);
        self::assertSame(['a', 'b'], $settings->list('ALLOWED_ORIGINS'));
        self::assertSame(self::root() . '/storage/x', $settings->path('FILES_PATH'));
        self::assertSame('C:/abs/k.json', $settings->path('KEYPAIR_PATH'));
        self::assertSame('', $settings->get('QR_PROXY_SECRET'), 'trust no proxy by default');
        self::assertSame(120, $settings->int('ACCESS_MAX_PER_IP_HOUR'));
        self::assertSame(['a', 'b'], $settings->allowedOrigins(), 'no dev origins outside a local php -S server');
    }

    public function testRootDotEnvOnlyFeedsTheStorageKeys(): void
    {
        // the repository-root .env is the owner's production checklist: only the bunny / CDN keys may come from it
        $file = $this->dir . '/.env';
        file_put_contents($file, implode("\n", [
            '# production values',
            'DB_HOST=db.production.example',
            'AUTH_SECRET=production-secret-production-secret',
            'SMTP_PASS="x"',
            'export BUNNY_STORAGE_ZONE="scutoons"   # the zone',
            "BUNNY_STORAGE_PASSWORD='pa#ss word'",
            'BUNNY_STORAGE_ENDPOINT=storage.bunnycdn.com # Frankfurt',
            'VITE_CDN_URL=https://cdn.scutoons.com',
            'BUNNY_API_KEY=              # may stay empty',
            'CDN_PUBLIC_URL=https://cdn.example',
        ]));
        $values = Settings::readDotEnv($file);
        self::assertSame(['BUNNY_STORAGE_ZONE', 'BUNNY_STORAGE_PASSWORD', 'BUNNY_STORAGE_ENDPOINT', 'BUNNY_API_KEY', 'CDN_PUBLIC_URL'], array_keys($values));
        self::assertSame(['scutoons', 'pa#ss word', 'storage.bunnycdn.com', '', 'https://cdn.example'], array_values($values));
        self::assertSame([], Settings::readDotEnv($this->dir . '/missing.env'));

        /** @var array<string, string> $defaults */
        $defaults = require self::root() . '/config/settings.php';
        $settings = new Settings(self::root(), $defaults, false, $values);
        self::assertSame('localhost', $settings->get('DB_HOST'), 'never the production database');
        self::assertSame('', $settings->get('AUTH_SECRET'));
        self::assertSame('', $settings->get('SMTP_PASS'));
        self::assertSame('scutoons', $settings->get('BUNNY_STORAGE_ZONE'));
        self::assertSame('https://cdn.example', $settings->get('CDN_PUBLIC_URL'));
        self::assertSame('', $settings->get('BUNNY_API_KEY'), 'empty falls back to the default');
        putenv('BUNNY_STORAGE_ZONE=from-the-environment');
        try {
            self::assertSame('from-the-environment', $settings->get('BUNNY_STORAGE_ZONE'), 'the real environment wins over the file');
        } finally {
            putenv('BUNNY_STORAGE_ZONE');
        }
    }

    public function testDevOriginsComeFromTheDevPortsFile(): void
    {
        $origins = Settings::devOrigins(dirname(self::root()) . '/shared/dev-ports.json');
        self::assertContains('http://localhost:13000', $origins, 'the embassy, without a config.local.php entry');
        self::assertContains('http://127.0.0.1:8000', $origins);
        self::assertSame([], Settings::devOrigins(self::root() . '/no-such-file.json'), 'production has no shared/ folder');
    }

    public function testMigrationsKeepTheirRecordedNames(): void
    {
        $runner = new MigrationRunner($this->db, self::root() . '/migrations');
        self::assertSame([], array_keys(array_filter($runner->status(), fn($applied) => $applied === null)), 'the template is fully migrated');
        self::assertContains('2026-09-29-accounts-inventory.mysql.sql', $runner->files());
        self::assertSame([], $runner->migrate(static fn() => null), 're-running applies nothing');
        // the test database itself was built from the base schema + every migration (MysqlTestDatabase)
        self::assertFalse($runner->needsBaseSchema());
        self::assertTrue($runner->tableExists('content_versions'));
        self::assertTrue($runner->tableExists('settings'));
    }

    /**
     * Every text column is utf8mb4, although the test database's default is latin1 like
     * production's server (character_set_server = latin1): a CREATE TABLE without its own charset would be latin1.
     */
    public function testEveryTextColumnIsUtf8mb4(): void
    {
        $rows = $this->db->all("SELECT CONCAT(TABLE_NAME, '.', COLUMN_NAME, ' ', CHARACTER_SET_NAME) AS c FROM information_schema.COLUMNS
            WHERE TABLE_SCHEMA = DATABASE() AND CHARACTER_SET_NAME IS NOT NULL AND CHARACTER_SET_NAME <> 'utf8mb4'");
        self::assertSame([], array_column($rows, 'c'));
        $tables = $this->db->all("SELECT TABLE_NAME AS t FROM information_schema.TABLES WHERE TABLE_SCHEMA = DATABASE() AND TABLE_TYPE = 'BASE TABLE' AND TABLE_COLLATION NOT LIKE 'utf8mb4%'");
        self::assertSame([], array_column($tables, 't'));
    }

    /**
     * The API sets its session sql_mode (DB_SQL_MODE, strict by default) whatever the server's
     * default is (production: NO_ENGINE_SUBSTITUTION) – a too long value fails instead of being cut silently.
     * DB_SQL_MODE=server keeps the server's mode.
     */
    public function testTheSessionSqlModeIsStrictByDefault(): void
    {
        $connect = fn(array $override): Database => \Scutoons\Kernel::container(Settings::fixed(self::root(), $override + $this->settings()))->get(Database::class);
        $strict = $connect(['DB_SQL_MODE' => 'STRICT_TRANS_TABLES,ERROR_FOR_DIVISION_BY_ZERO,NO_ENGINE_SUBSTITUTION']);
        self::assertStringContainsString('STRICT_TRANS_TABLES', (string) $strict->value('SELECT @@SESSION.sql_mode'));
        try {
            $strict->run("INSERT INTO code_events (code, event, created_at) VALUES ('ninechars', 'scan', '2026-01-01 00:00:00')");
            self::fail('a 9-character code went into VARCHAR(8)');
        } catch (\PDOException $e) {
            self::assertStringContainsString('1406', $e->getMessage(), 'Data too long');
        }
        $server = $connect(['DB_SQL_MODE' => 'server']);
        self::assertSame($server->value('SELECT @@GLOBAL.sql_mode'), $server->value('SELECT @@SESSION.sql_mode'));
        $defaults = require self::root() . '/config/settings.php';
        self::assertStringContainsString('STRICT_TRANS_TABLES', $defaults['DB_SQL_MODE'], 'strict is the default');
        $strict->close();
        $server->close();
    }

    public function testFileStoreWhitelist(): void
    {
        $files = new FileStore($this->dir . '/files');
        self::assertTrue(FileStore::validName('svgs', 'SCUTOONS-QR-MEMBERSHIP-abc.svg'));
        self::assertFalse(FileStore::validName('svgs', '.htaccess.svg'));
        self::assertFalse(FileStore::validName('svgs', '../x.svg'));
        self::assertFalse(FileStore::validName('keys', 'x.json'));
        $files->write('batch', 'BATCH-x.json', '{}');
        self::assertSame(['BATCH-x.json'], $files->list('batch', 'BATCH-*.json'));
    }

    public function testIds(): void
    {
        self::assertMatchesRegularExpression('/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/', Ids::uuid());
        self::assertLessThan(Ids::uuid7(2000), Ids::uuid7(1000), 'v7 sorts by time');
        self::assertMatchesRegularExpression('/^[A-Za-z0-9_-]{8}$/', Ids::nanoid());
        self::assertMatchesRegularExpression('/^[A-Za-z0-9_-]{43}$/', Ids::token());
    }
}
