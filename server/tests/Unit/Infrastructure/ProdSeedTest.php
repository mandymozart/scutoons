<?php
declare(strict_types=1);

namespace Scutoons\Tests\Infrastructure;

use RuntimeException;
use Scutoons\Infrastructure\Database\ProdSeed;
use Scutoons\Tests\TestCase;

/** The production seed of a local database (CUTOVER 10.30, docs/DATABASE.md "Production seed") */
final class ProdSeedTest extends TestCase
{
    /** A phpMyAdmin dump in miniature: structure, a seed table over two lines, tables that must never be loaded */
    private const DUMP = <<<'SQL'
        -- phpMyAdmin SQL Dump
        SET SQL_MODE = "NO_AUTO_VALUE_ON_ZERO";
        START TRANSACTION;
        CREATE TABLE `users` (`id` char(36) NOT NULL);
        INSERT INTO `users` (`id`, `email`) VALUES
        ('u1', 'someone@example.com');
        INSERT INTO `catalog_items` (`id`, `position`, `data`, `updated_at`, `updated_by`, `toon`, `edition_id`) VALUES
        ('onion', 0, '{"id":"onion","name":{"de":"Zwiebel – \\\"süß\\\"","en":"Onion"},"kind":"virtual","category":"food"}', '2026-10-05 10:00:00', 'admin:x', 'catpope', NULL),
        ('leek', 1, '{"id":"leek","note":"a line\\nbreak; and a );"}', '2026-10-05 10:00:00', 'admin:x', NULL, NULL);
        INSERT INTO `sessions` (`id`) VALUES ('s1');
        COMMIT;
        SQL;

    public function testExtractKeepsOnlyTheSeedTables(): void
    {
        $result = ProdSeed::extract(self::DUMP);
        self::assertCount(1, $result['statements']['catalog_items']);
        self::assertSame(['users' => 1, 'sessions' => 1], $result['skipped']);
        $file = ProdSeed::render($result['statements'], 'dump.sql');
        self::assertStringNotContainsString('someone@example.com', $file);
        self::assertStringNotContainsString('`sessions`', $file);
        self::assertSame($result['statements'], ProdSeed::extract($file)['statements'], 'the written file reads back the same');
    }

    public function testLoadReplacesTheSeedTablesOnce(): void
    {
        self::assertSame(27, (int) $this->db->value('SELECT COUNT(*) FROM catalog_items'), 'the migration\'s snapshot before');
        $first = ProdSeed::load($this->db, self::DUMP);
        self::assertTrue($first['loaded']);
        self::assertSame(2, $first['rows']['catalog_items']);
        self::assertSame(0, $first['rows']['catalog_recipes'], 'every seed table is emptied, also those the dump lacks');
        self::assertSame(['onion', 'leek'], array_column($this->db->all('SELECT id FROM catalog_items ORDER BY position'), 'id'));
        self::assertSame('Zwiebel – "süß"', json_decode((string) $this->db->value("SELECT data FROM catalog_items WHERE id = 'onion'"), true)['name']['de']);
        self::assertSame("a line\nbreak; and a );", json_decode((string) $this->db->value("SELECT data FROM catalog_items WHERE id = 'leek'"), true)['note']);
        self::assertSame(0, (int) $this->db->value('SELECT COUNT(*) FROM users'), 'never other tables');

        $this->db->run("DELETE FROM catalog_items WHERE id = 'leek'"); // a local edit
        self::assertFalse(ProdSeed::load($this->db, self::DUMP)['loaded'], 'the same file again: nothing happens');
        self::assertSame(1, (int) $this->db->value('SELECT COUNT(*) FROM catalog_items'), 'local edits stay');
        self::assertTrue(ProdSeed::load($this->db, self::DUMP, true)['loaded'], 'forced');
        self::assertSame(2, (int) $this->db->value('SELECT COUNT(*) FROM catalog_items'));
    }

    public function testAFileWithoutSeedRowsIsRefused(): void
    {
        $this->expectException(RuntimeException::class);
        ProdSeed::load($this->db, "INSERT INTO `users` (`id`) VALUES ('u1');\n");
    }
}
