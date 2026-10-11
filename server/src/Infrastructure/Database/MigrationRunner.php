<?php
declare(strict_types=1);

namespace Scutoons\Infrastructure\Database;

use Throwable;

/**
 * Applies the dated SQL migrations in migrations/mysql/*.mysql.sql (MariaDB) in name order and records
 * each in `schema_migrations` by its file name – the same names the old scripts/migrate.php recorded, so an existing
 * database re-runs nothing. A database without the `qr` table first gets the base schema
 * (migrations/schema/qr.mysql.sql). `ALTER TABLE x ADD COLUMN y` is skipped when the column exists, so re-running
 * a file is safe. migrations/mysql/legacy/ holds the two MySQL-only changes from before the account system; they are
 * never run (edition_id is in the base schema; template_data was never applied in production – checked 2026-10-05 –
 * and is not in the base schema either).
 *
 * Production (shared host, no shell): the developer plans and applies the pending files through the API (BACKLOG 10.61,
 * Domain\Dev\Migrations): plan() is a dry run – a file of data statements runs in a transaction that is rolled back,
 * to count the rows; a file with DDL (implicit commit) is only listed. applyFile() runs one file – data-only files in a
 * transaction, others statement by statement – and records it.
 */
final class MigrationRunner
{
    /** the folder / file suffix of the migrations (the names schema_migrations records) */
    public const DRIVER = 'mysql';
    /** how much of a statement a plan shows */
    private const SHOWN = 400;

    public function __construct(private readonly Database $db, private readonly string $directory)
    {
    }

    /** @return list<string> file names of the migrations, sorted */
    public function files(): array
    {
        $driver = self::DRIVER;
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

    /** @return list<string> the pending file names, in order */
    public function pending(): array
    {
        return array_keys(array_filter($this->status(), fn(?string $appliedAt) => $appliedAt === null));
    }

    /** SHA-256 of a migration file's content (a plan names the exact files it was made from) */
    public function sha256(string $file): string
    {
        return hash_file('sha256', $this->path($file)) ?: '';
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
        $driver = self::DRIVER;
        if ($this->needsBaseSchema()) {
            $count = $this->runFile("{$this->directory}/schema/qr.$driver.sql");
            $log("  schema   qr.$driver.sql ($count statements)");
        }
        $applied = [];
        foreach ($this->pending() as $file) {
            [$count, $skipped] = $this->runMigration($this->path($file));
            $this->record($file);
            $log("  applied  $file ($count statements, $skipped skipped)");
            $applied[] = $file;
        }
        return $applied;
    }

    /**
     * The dry run of the pending files: per file its statements with their kind (SqlStatement), and for a file of data
     * statements only the rows each one would change – run in a transaction that is rolled back. Each file is counted on
     * its own, before the files above it ran (a data file that needs a pending DDL file above it may fail here).
     * @return list<array{name: string, sha256: string, kind: string, dry_run: bool, error: string|null, statements: list<array{sql: string, kind: string, rows: int|null, skipped: bool, backup: bool}>}>
     */
    public function plan(): array
    {
        $plans = [];
        foreach ($this->pending() as $file) {
            $statements = [];
            $kinds = [];
            foreach (self::statements($this->path($file)) as $sql) {
                $kind = SqlStatement::kind($sql);
                $kinds[$kind] = true;
                $statements[] = [
                    'sql' => mb_strlen($sql) > self::SHOWN ? mb_substr($sql, 0, self::SHOWN) . ' …' : $sql,
                    'kind' => $kind,
                    'rows' => null,
                    'skipped' => $kind === SqlStatement::DDL && $this->skippable($sql),
                    'backup' => SqlStatement::backupSelect($sql) !== null,
                    'full' => $sql,
                ];
            }
            $fileKind = count($kinds) > 1 ? 'mixed' : (array_key_first($kinds) ?? SqlStatement::DATA);
            $error = null;
            if ($fileKind === SqlStatement::DATA) {
                try {
                    $this->dryRun($statements);
                } catch (Throwable $e) {
                    $error = $e->getMessage();
                }
            }
            $plans[] = [
                'name' => $file,
                'sha256' => $this->sha256($file),
                'kind' => $fileKind,
                'dry_run' => $fileKind === SqlStatement::DATA,
                'error' => $error,
                'statements' => array_map(function (array $s) {
                    unset($s['full']);
                    return $s;
                }, $statements),
            ];
        }
        return $plans;
    }

    /**
     * Runs one migration file and records it: data statements only → in one transaction (all or nothing); with DDL or
     * other statements → one after another (what ran before an error stays – the files are idempotent, so the file is
     * run again once fixed). $before sees each statement right before it runs (the caller backs up its rows).
     * @param callable(string): void $log
     * @param (callable(string, int): void)|null $before (the statement, its index)
     * @return array{statements: int, skipped: int, rows: int, data: bool}
     */
    public function applyFile(string $file, callable $log, ?callable $before = null): array
    {
        $statements = self::statements($this->path($file));
        $kinds = array_unique(array_map(fn(string $s) => SqlStatement::kind($s), $statements));
        $dataOnly = $kinds === [SqlStatement::DATA];
        $result = ['statements' => count($statements), 'skipped' => 0, 'rows' => 0, 'data' => in_array(SqlStatement::DATA, $kinds, true)];
        $work = function () use ($statements, $log, $before, &$result): void {
            foreach ($statements as $i => $sql) {
                if (SqlStatement::kind($sql) === SqlStatement::DDL && $this->skippable($sql)) {
                    $result['skipped']++;
                    $log(sprintf('    %d skipped (the column exists)', $i + 1));
                    continue;
                }
                if ($before !== null) $before($sql, $i);
                $rows = $this->db->pdo()->exec($sql);
                $result['rows'] += max(0, (int) $rows);
                $log(sprintf('    %d %s: %d row(s)', $i + 1, SqlStatement::kind($sql), (int) $rows));
            }
        };
        if ($dataOnly) $this->db->transaction($work);
        else $work();
        $this->record($file);
        return $result;
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

    /** Marks a file as applied (INSERT IGNORE: a migration may record itself – 2026-10-11-developer-role does) */
    public function record(string $file): void
    {
        $this->ensureTable();
        $this->db->run('INSERT IGNORE INTO schema_migrations (name, applied_at) VALUES (?, ?)', [$file, gmdate('Y-m-d H:i:s')]);
    }

    /**
     * Data statements in a transaction that is always rolled back; sets each one's `rows`
     * @param list<array{sql: string, kind: string, rows: int|null, skipped: bool, backup: bool, full: string}> $statements
     */
    private function dryRun(array &$statements): void
    {
        $pdo = $this->db->pdo();
        $pdo->beginTransaction();
        try {
            foreach ($statements as &$statement) {
                $statement['rows'] = max(0, (int) $pdo->exec($statement['full']));
            }
            unset($statement);
        } finally {
            if ($pdo->inTransaction()) $pdo->rollBack();
        }
    }

    private function path(string $file): string
    {
        if (preg_match('/^[\w.-]+\.' . self::DRIVER . '\.sql$/', $file) !== 1) throw new \InvalidArgumentException("Not a migration file name: $file");
        return "{$this->directory}/" . self::DRIVER . "/$file";
    }

    /** @return array{0: int, 1: int} [statements, skipped ADD COLUMNs] */
    private function runMigration(string $file): array
    {
        $statements = self::statements($file);
        $skipped = 0;
        foreach ($statements as $statement) {
            if ($this->skippable($statement)) {
                $skipped++;
                continue;
            }
            $this->db->exec($statement);
        }
        return [count($statements), $skipped];
    }

    /** `ALTER TABLE x ADD COLUMN y` of a column that exists */
    private function skippable(string $statement): bool
    {
        return preg_match('/^ALTER\s+TABLE\s+`?(\w+)`?\s+ADD\s+COLUMN\s+(?:IF\s+NOT\s+EXISTS\s+)?`?(\w+)`?/i', $statement, $m) === 1
            && $this->tableExists($m[1]) && $this->hasColumn($m[1], $m[2]);
    }

    private function ensureTable(): void
    {
        $this->db->exec('CREATE TABLE IF NOT EXISTS `schema_migrations` (`name` VARCHAR(191) NOT NULL, `applied_at` DATETIME NOT NULL, PRIMARY KEY (`name`)) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci');
    }

    public function tableExists(string $table): bool
    {
        return $this->db->value('SHOW TABLES LIKE ' . $this->db->pdo()->quote($table)) !== null;
    }

    private function hasColumn(string $table, string $column): bool
    {
        $rows = $this->db->all("SHOW COLUMNS FROM `$table`");
        foreach ($rows as $row) {
            if (strcasecmp((string) ($row['Field'] ?? ''), $column) === 0) return true;
        }
        return false;
    }
}
