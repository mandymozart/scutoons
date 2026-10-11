<?php
declare(strict_types=1);

namespace Scutoons\Tests\Infrastructure;

use PHPUnit\Framework\Attributes\DataProvider;
use PHPUnit\Framework\TestCase;
use Scutoons\Infrastructure\Database\SqlStatement;

/** What a migration statement is and which rows it changes (BACKLOG 10.61: dry run, backups) */
final class SqlStatementTest extends TestCase
{
    /** @return array<string, array{0: string, 1: string, 2: string|null}> statement, kind, backup SELECT */
    public static function statements(): array
    {
        return [
            'subqueries keep their WHERE' => [
                "UPDATE `entity_versions` SET json = JSON_SET(json, '$.a', (SELECT x FROM t WHERE y = 1)) WHERE kind = 'map' AND id IN (SELECT id FROM z WHERE q) LIMIT 3",
                'data', "SELECT * FROM `entity_versions` WHERE kind = 'map' AND id IN (SELECT id FROM z WHERE q)",
            ],
            'an alias, a WHERE in a string' => ["UPDATE users u SET u.x = 'it''s WHERE' WHERE u.id = 'a'", 'data', "SELECT * FROM `users` AS `u` WHERE u.id = 'a'"],
            'every row' => ['UPDATE catalog_items SET x = 1', 'data', 'SELECT * FROM `catalog_items`'],
            'a join' => ['UPDATE a JOIN b ON a.id = b.id SET a.x = 1', 'data', null],
            'two tables' => ['UPDATE a, b SET a.x = b.x', 'data', null],
            'delete' => ["DELETE FROM sessions WHERE created_at < '2020-01-01' ORDER BY id", 'data', "SELECT * FROM `sessions` WHERE created_at < '2020-01-01'"],
            'multi-table delete' => ['DELETE a FROM a JOIN b ON a.id = b.id', 'data', null],
            'delete using' => ['DELETE FROM t USING t JOIN u ON t.id = u.id', 'data', null],
            'insert' => ['INSERT INTO t (a) VALUES (1)', 'data', ''],
            'upsert' => ['INSERT INTO t (a) VALUES (1) ON DUPLICATE KEY UPDATE a = 2', 'data', null],
            'replace' => ['REPLACE INTO t VALUES (1)', 'data', null],
            'a variable' => ['SET @x = 1', 'data', ''],
            'session setting' => ['SET NAMES utf8mb4', 'other', ''],
            'ddl' => ['ALTER TABLE t ADD COLUMN c INT', 'ddl', null],
            'commit' => ['COMMIT', 'other', null],
        ];
    }

    #[DataProvider('statements')]
    public function testKindAndBackup(string $sql, string $kind, ?string $backup): void
    {
        self::assertSame([$kind, $backup], [SqlStatement::kind($sql), SqlStatement::backupSelect($sql)]);
    }

    public function testTableOfABackup(): void
    {
        self::assertSame('users', SqlStatement::table('SELECT * FROM `users` AS `u` WHERE 1'));
    }
}
