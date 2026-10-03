<?php
declare(strict_types=1);

namespace Scutoons\Tests;

use RuntimeException;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Infrastructure\Database\MigrationRunner;

/**
 * The PHPUnit suite on MariaDB/MySQL (the production engine) instead of SQLite: `vendor/bin/phpunit -c
 * phpunit.mysql.xml` (TEST_DB_* below; docs/DATABASE.md "Local MariaDB"). Once per run the test database is emptied
 * (every table dropped) and migrated with migrations/mysql – a fresh production-like schema. Its rows after that (seed
 * rows of the migrations) are kept in memory. Before each test the tables a test touched are truncated (AUTO_INCREMENT
 * back to the start, like a fresh SQLite copy) and the seed rows written again; tables with seed rows always are.
 * Refuses a database whose name does not end in "test" (it drops every table).
 */
final class MysqlTestDatabase
{
    private static ?Database $db = null;
    /** @var array<string, list<array<string, mixed>>> table → rows right after the migrations */
    private static array $seed = [];
    /** @var array<string, int|null> table → AUTO_INCREMENT right after the migrations */
    private static array $autoIncrement = [];

    /**
     * TEST_DB_SQL_MODE: the API's DB_SQL_MODE for this run (unset = its default, strict; `server` = the server's own
     * mode – production's is not strict: `TEST_DB_SQL_MODE=server` runs the suite the way the host would behave alone)
     * @return array{host: string, port: string, name: string, user: string, pass: string, sql_mode: string|null}|null null = SQLite
     */
    public static function config(): ?array
    {
        if (strtolower((string) getenv('TEST_DB_DRIVER')) !== 'mysql') return null;
        $env = static fn(string $key, string $default): string => (string) (getenv($key) ?: $default);
        return [
            'host' => $env('TEST_DB_HOST', '127.0.0.1'),
            'port' => $env('TEST_DB_PORT', '3306'),
            'name' => $env('TEST_DB_NAME', 'scutoons_test'),
            'user' => $env('TEST_DB_USER', 'scutoons'),
            'pass' => $env('TEST_DB_PASS', ''),
            'sql_mode' => getenv('TEST_DB_SQL_MODE') !== false && getenv('TEST_DB_SQL_MODE') !== '' ? (string) getenv('TEST_DB_SQL_MODE') : null,
        ];
    }

    /** Brings the test database back to "freshly migrated" (migrates on the first call of the run) */
    public static function reset(string $root): void
    {
        $db = self::$db ?? self::prepare($root);
        $auto = self::autoIncrements($db);
        if (array_keys($auto) != array_keys(self::$autoIncrement)) {
            // a test dropped a table ("works before the migration"): build the schema again
            self::migrateFresh($db, $root);
            return;
        }
        $tables = array_keys(self::$autoIncrement);
        $dirty = [];
        $counts = [];
        foreach (array_chunk($tables, 60) as $chunk) {
            $sql = implode(' UNION ALL ', array_map(static fn(string $t) => "SELECT '$t' AS t, COUNT(*) AS c FROM `$t`", $chunk));
            foreach ($db->all($sql) as $row) $counts[(string) $row['t']] = (int) $row['c'];
        }
        foreach ($tables as $table) {
            if (self::$seed[$table] !== [] || $counts[$table] > 0 || $auto[$table] !== self::$autoIncrement[$table]) $dirty[] = $table;
        }
        if ($dirty === []) return;
        $db->exec('SET FOREIGN_KEY_CHECKS = 0');
        try {
            foreach ($dirty as $table) {
                // DELETE, not TRUNCATE: TRUNCATE recreates the InnoDB table (slow, ~3x the suite's run time)
                $db->exec("DELETE FROM `$table`");
                foreach (self::$seed[$table] as $row) {
                    $columns = implode(', ', array_map(static fn(string $c) => "`$c`", array_keys($row)));
                    $db->run("INSERT INTO `$table` ($columns) VALUES (" . implode(', ', array_fill(0, count($row), '?')) . ')', array_values($row));
                }
                // AUTO_INCREMENT back to where the migrations left it (ids start like on a fresh SQLite copy)
                $start = self::$autoIncrement[$table];
                if ($start !== null) $db->exec("ALTER TABLE `$table` AUTO_INCREMENT = $start");
            }
        } finally {
            $db->exec('SET FOREIGN_KEY_CHECKS = 1');
        }
    }

    private static function prepare(string $root): Database
    {
        $config = self::config() ?? throw new RuntimeException('TEST_DB_DRIVER is not mysql');
        if (!str_ends_with($config['name'], 'test')) {
            throw new RuntimeException("TEST_DB_NAME '{$config['name']}' must end in 'test': the suite drops every table of it");
        }
        if (!extension_loaded('pdo_mysql')) {
            throw new RuntimeException('pdo_mysql is not loaded: php -d extension=pdo_mysql vendor/bin/phpunit -c phpunit.mysql.xml (or enable it in php.ini)');
        }
        $db = Database::mysql($config['host'], $config['name'], $config['user'], $config['pass'], $config['port']);
        self::migrateFresh($db, $root);
        return self::$db = $db;
    }

    /** Drops every table of the test database, runs the migrations, remembers the resulting rows */
    private static function migrateFresh(Database $db, string $root): void
    {
        $db->exec('SET FOREIGN_KEY_CHECKS = 0');
        foreach ($db->all('SELECT TABLE_NAME AS t, TABLE_TYPE AS k FROM information_schema.TABLES WHERE TABLE_SCHEMA = DATABASE()') as $row) {
            $db->exec(($row['k'] === 'VIEW' ? 'DROP VIEW' : 'DROP TABLE') . " IF EXISTS `{$row['t']}`");
        }
        $db->exec('SET FOREIGN_KEY_CHECKS = 1');
        self::$seed = [];
        (new MigrationRunner($db, $root . '/migrations'))->migrate(static fn() => null);
        foreach ($db->all("SELECT TABLE_NAME AS t FROM information_schema.TABLES WHERE TABLE_SCHEMA = DATABASE() AND TABLE_TYPE = 'BASE TABLE'") as $row) {
            $table = (string) $row['t'];
            self::$seed[$table] = $db->all("SELECT * FROM `$table`");
        }
        self::$autoIncrement = self::autoIncrements($db);
    }

    /** @return array<string, int|null> table → its next AUTO_INCREMENT value (null = no AUTO_INCREMENT column) */
    private static function autoIncrements(Database $db): array
    {
        $auto = [];
        $rows = $db->all("SELECT TABLE_NAME AS t, AUTO_INCREMENT AS a FROM information_schema.TABLES WHERE TABLE_SCHEMA = DATABASE() AND TABLE_TYPE = 'BASE TABLE' ORDER BY TABLE_NAME");
        foreach ($rows as $row) $auto[(string) $row['t']] = $row['a'] === null ? null : (int) $row['a'];
        return $auto;
    }
}
