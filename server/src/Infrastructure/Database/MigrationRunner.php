<?php
declare(strict_types=1);

namespace Scutoons\Infrastructure\Database;


/**
 * Applies the dated SQL migrations in migrations/<driver>/*.<driver>.sql (sqlite | mysql) in name order and records
 * each in `schema_migrations` by its file name – the same names the old scripts/migrate.php recorded, so an existing
 * database re-runs nothing. A database without the `qr` table first gets the base schema
 * (migrations/schema/qr.<driver>.sql). `ALTER TABLE x ADD COLUMN y` is skipped when the column exists, so re-running
 * a file is safe. migrations/mysql/legacy/ holds the two MySQL-only changes from before the account system; they are
 * never run (edition_id is in the base schema; template_data was never applied in production – checked 2026-10-05 –
 * and is not in the base schema either).
 *
 * Production (shared host, no shell): run the pending *.mysql.sql files in phpMyAdmin instead – they are idempotent.
 */
final class MigrationRunner
{
    public function __construct(private readonly Database $db, private readonly string $directory)
    {
    }

    /** @return list<string> file names of this driver's migrations, sorted */
    public function files(): array
    {
        $driver = $this->db->driver();
        $files = array_map('basename', glob("{$this->directory}/$driver/*.$driver.sql") ?: []);
        sort($files);
        return $files;
    }

    /** @return array<string, string|null> file name → applied_at (null = pending) */
    public function status(): array
    {
        $this->ensureTable();
        $applied = [];
        foreach ($this->db->all('SELECT name, applied_at FROM schema_migrations') as $row) $applied[(string) $row['name']] = (string) $row['applied_at'];
        $status = [];
        foreach ($this->files() as $file) $status[$file] = $applied[$file] ?? null;
        return $status;
    }

    public function needsBaseSchema(): bool
    {
        return !$this->tableExists('qr');
    }

    /**
     * Applies the base schema (fresh database) and every pending migration
     * @param callable(string): void $log
     * @return list<string> the applied file names
     */
    public function migrate(callable $log): array
    {
        $driver = $this->db->driver();
        if ($this->needsBaseSchema()) {
            $count = $this->runFile("{$this->directory}/schema/qr.$driver.sql");
            $log("  schema   qr.$driver.sql ($count statements)");
        }
        $applied = [];
        foreach ($this->status() as $file => $appliedAt) {
            if ($appliedAt !== null) continue;
            [$count, $skipped] = $this->runMigration("{$this->directory}/$driver/$file");
            $this->db->run('INSERT INTO schema_migrations (name, applied_at) VALUES (?, ?)', [$file, gmdate('Y-m-d H:i:s')]);
            $log("  applied  $file ($count statements, $skipped skipped)");
            $applied[] = $file;
        }
        return $applied;
    }

    /** @return list<string> the statements of a SQL file (comments removed; statements end with ";" + newline) */
    public static function statements(string $file): array
    {
        $sql = (string) preg_replace('/^\s*--.*$/m', '', (string) file_get_contents($file));
        return array_values(array_filter(array_map('trim', preg_split('/;\s*(\r?\n|$)/', $sql) ?: []), fn(string $s) => $s !== ''));
    }

    /** Runs every statement of a file (the base schema); returns the count */
    public function runFile(string $file): int
    {
        $statements = self::statements($file);
        foreach ($statements as $statement) $this->db->exec($statement);
        return count($statements);
    }

    /** @return array{0: int, 1: int} [statements, skipped ADD COLUMNs] */
    private function runMigration(string $file): array
    {
        $statements = self::statements($file);
        $skipped = 0;
        foreach ($statements as $statement) {
            if (preg_match('/^ALTER\s+TABLE\s+`?(\w+)`?\s+ADD\s+COLUMN\s+(?:IF\s+NOT\s+EXISTS\s+)?`?(\w+)`?/i', $statement, $m)
                && $this->hasColumn($m[1], $m[2])) {
                $skipped++;
                continue;
            }
            $this->db->exec($statement);
        }
        return [count($statements), $skipped];
    }

    private function ensureTable(): void
    {
        $this->db->exec($this->db->isSqlite()
            ? 'CREATE TABLE IF NOT EXISTS schema_migrations (name TEXT NOT NULL PRIMARY KEY, applied_at TEXT NOT NULL)'
            : 'CREATE TABLE IF NOT EXISTS `schema_migrations` (`name` VARCHAR(191) NOT NULL, `applied_at` DATETIME NOT NULL, PRIMARY KEY (`name`)) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci');
    }

    public function tableExists(string $table): bool
    {
        return $this->db->isSqlite()
            ? $this->db->value("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?", [$table]) !== null
            : $this->db->value('SHOW TABLES LIKE ' . $this->db->pdo()->quote($table)) !== null;
    }

    private function hasColumn(string $table, string $column): bool
    {
        $rows = $this->db->isSqlite()
            ? $this->db->all("PRAGMA table_info($table)")
            : $this->db->all("SHOW COLUMNS FROM `$table`");
        foreach ($rows as $row) {
            if (strcasecmp((string) ($row['name'] ?? $row['Field'] ?? ''), $column) === 0) return true;
        }
        return false;
    }
}
