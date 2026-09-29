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
        self::assertSame('qr.scutoons.com', $settings->get('QR_HOST'));
        self::assertTrue($settings->bool('QR_HOST_SERVES_API'));
        self::assertSame(120, $settings->int('ACCESS_MAX_PER_IP_HOUR'));
    }

    public function testMigrationsKeepTheirRecordedNames(): void
    {
        $runner = new MigrationRunner($this->db, self::root() . '/migrations');
        self::assertSame([], array_keys(array_filter($runner->status(), fn($applied) => $applied === null)), 'the template is fully migrated');
        self::assertContains('2026-09-29-accounts-inventory.sqlite.sql', $runner->files());
        self::assertSame([], $runner->migrate(static fn() => null), 're-running applies nothing');

        $fresh = $this->dir . '/fresh.sqlite';
        $freshRunner = new MigrationRunner(Database::sqlite($fresh), self::root() . '/migrations');
        self::assertTrue($freshRunner->needsBaseSchema());
        self::assertCount(5, $freshRunner->migrate(static fn() => null));
        self::assertTrue($freshRunner->tableExists('content_versions'));
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
