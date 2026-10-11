<?php
declare(strict_types=1);

namespace Scutoons\Domain\Dev;

use Scutoons\Domain\ApiError;
use Scutoons\Infrastructure\Cache\Cache;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Infrastructure\Database\MigrationRunner;
use Scutoons\Infrastructure\Database\SqlStatement;
use Scutoons\Support\Ids;
use Throwable;

/**
 * Migrations through the API (BACKLOG 10.61, owner 2026-10-11) instead of SQL pasted into phpMyAdmin – triggered by the
 * deploy (POST /api/deploy/migrate, MIGRATE_KEY) or by a developer's agent over MCP (role developer, their normal AI
 * connection). Only the files that came with the deploy run (server/migrations/mysql/), never SQL from a request.
 *
 *   plan  – a dry run of the pending files (MigrationRunner::plan), stored in dev_migration_runs with the files' SHA-256;
 *           older open plans go stale
 *   apply – only a plan of the last PLAN_TTL whose files are still exactly the pending ones; one run at a time
 *           (GET_LOCK); the rows each UPDATE / DELETE changes are backed up first (JSON lines below
 *           MIGRATION_BACKUP_PATH); afterwards the library cache is emptied (a data migration changes stored versions in
 *           place, which the content-addressed cache can't see)
 *   run   – plan + apply in one go (the deploy; an agent that doesn't need to look first)
 */
final class Migrations
{
    public const PLAN_TTL = 86400;
    private const LOCK = 'scutoons_migrate';
    private const STATUSES = ['planned', 'applied', 'failed', 'stale'];

    public function __construct(
        private readonly Database $db,
        private readonly Clock $clock,
        private readonly Settings $settings,
        private readonly Cache $cache,
    ) {
    }

    /** @return array<string, mixed> the database, every migration file (applied or pending) and the latest runs */
    public function status(): array
    {
        $runner = $this->runner();
        $files = [];
        foreach ($runner->status() as $name => $appliedAt) {
            $files[] = ['name' => $name, 'applied_at' => $appliedAt, 'sha256' => $runner->sha256($name)];
        }
        return [
            'database' => (string) $this->db->value('SELECT DATABASE()'),
            'base_schema' => !$runner->needsBaseSchema(),
            'pending' => count(array_filter($files, fn(array $f) => $f['applied_at'] === null)),
            'files' => $files,
            'runs' => array_map(fn(array $r) => $this->summary($r),
                $this->db->all('SELECT * FROM dev_migration_runs ORDER BY created_at DESC LIMIT 10')),
        ];
    }

    /**
     * A dry run of the pending files, stored as a plan to apply
     * @param array<string, mixed>|null $user null = the deploy
     * @return array<string, mixed>
     */
    public function plan(?array $user, string $via): array
    {
        $runner = $this->runner();
        if ($runner->needsBaseSchema()) throw new ApiError(409, 'no_schema', 'This database has no base schema – set it up with bin/console migrate.');
        $files = $this->locked(fn() => $runner->plan());
        $id = Ids::uuid7();
        $now = $this->clock->utc();
        $this->db->run("UPDATE dev_migration_runs SET status = 'stale' WHERE status = 'planned'");
        $this->db->run('INSERT INTO dev_migration_runs (id, status, plan, created_by, created_via, created_at) VALUES (?, ?, ?, ?, ?, ?)',
            [$id, 'planned', json_encode(['files' => $files], JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_THROW_ON_ERROR),
                $user !== null ? (string) $user['id'] : null, mb_substr($via, 0, 120), $now]);
        return $this->get($id);
    }

    /**
     * Plan + apply in one go; null when nothing is pending (no run is stored then)
     * @param array<string, mixed>|null $user null = the deploy
     * @return array<string, mixed>|null
     */
    public function run(?array $user, string $via): ?array
    {
        if ($this->runner()->pending() === []) return null;
        $run = $this->plan($user, $via);
        return $this->apply((string) $run['id'], $user);
    }

    /** @return array<string, mixed> one run with its plan and log */
    public function get(string $id): array
    {
        return $this->full($this->row($id));
    }

    /**
     * Applies a plan: 409 not_planned (applied, failed or stale already) · 409 plan_stale (older than PLAN_TTL, or the
     * pending files are not exactly the planned ones any more – plan again) · 409 busy (another run is going). A file
     * that fails ends the run (`failed`, the error and the log stored; a data-only file is rolled back whole).
     * @param array<string, mixed>|null $user null = the deploy
     * @return array<string, mixed>
     */
    public function apply(string $id, ?array $user): array
    {
        $row = $this->row($id);
        if ($row['status'] !== 'planned') throw new ApiError(409, 'not_planned', "This plan is {$row['status']} – make a new one.");
        if ((string) $row['created_at'] < $this->clock->utc(-self::PLAN_TTL)) {
            $this->setStatus($id, 'stale');
            throw new ApiError(409, 'plan_stale', 'This plan is older than a day – make a new one.');
        }
        @set_time_limit(300);
        return $this->locked(function () use ($id, $row, $user) {
            $runner = $this->runner();
            $planned = array_map(fn(array $f) => [(string) $f['name'], (string) $f['sha256']], $this->decode($row)['files']);
            $current = array_map(fn(string $name) => [$name, $runner->sha256($name)], $runner->pending());
            if ($planned !== $current) {
                $this->setStatus($id, 'stale');
                throw new ApiError(409, 'plan_stale', 'The pending migrations changed since this plan – make a new one.');
            }
            $log = [];
            $say = function (string $line) use (&$log): void {
                $log[] = $line;
            };
            $backup = $this->backupFile($id);
            $data = false;
            $status = 'applied';
            $error = null;
            try {
                foreach ($planned as [$name]) {
                    $say($name);
                    $result = $runner->applyFile($name, $say, fn(string $sql, int $i) => $this->backup($backup, $name, $i, $sql, $say));
                    $data = $data || $result['data'];
                    $say(sprintf('  done: %d statement(s), %d skipped, %d row(s)', $result['statements'], $result['skipped'], $result['rows']));
                }
            } catch (Throwable $e) {
                $status = 'failed';
                $error = $e->getMessage();
                $say('  FAILED: ' . $error);
            }
            if ($data) {
                $removed = $this->cache->clear();
                $say('library cache emptied' . ($removed >= 0 ? " ($removed entries)" : ''));
            }
            $this->db->run('UPDATE dev_migration_runs SET status = ?, applied_by = ?, applied_at = ?, log = ?, error = ?, backup = ? WHERE id = ?',
                [$status, $user !== null ? (string) $user['id'] : null, $this->clock->utc(), implode("\n", $log), $error, is_file($backup) ? $this->relative($backup) : null, $id]);
            error_log("[qr-api] migrations $id $status by " . ($user !== null ? (string) ($user['email'] ?? $user['id']) : 'the deploy') . ($error !== null ? ": $error" : ''));
            return $this->get($id);
        });
    }

    /**
     * Writes the rows a statement is about to change to the run's backup (one JSON line per row)
     * @param callable(string): void $say
     */
    private function backup(string $file, string $migration, int $index, string $sql, callable $say): void
    {
        $select = SqlStatement::backupSelect($sql);
        if ($select === '') return;
        if ($select === null) {
            if (SqlStatement::kind($sql) === SqlStatement::DATA) $say(sprintf('    %d: no backup (several tables / REPLACE / ON DUPLICATE KEY)', $index + 1));
            return;
        }
        $dir = dirname($file);
        if (!is_dir($dir) && !@mkdir($dir, 0770, true) && !is_dir($dir)) throw new \RuntimeException("The backup folder $dir can't be made – nothing was run.");
        $out = fopen($file, 'ab') ?: throw new \RuntimeException("The backup file $file can't be written – nothing was run.");
        $count = 0;
        try {
            $table = SqlStatement::table($select);
            $statement = $this->db->run($select);
            while (($row = $statement->fetch()) !== false) {
                fwrite($out, json_encode(['migration' => $migration, 'statement' => $index + 1, 'table' => $table, 'row' => $row],
                    JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_INVALID_UTF8_SUBSTITUTE) . "\n");
                $count++;
            }
        } finally {
            fclose($out);
        }
        $say(sprintf('    %d: %d row(s) backed up', $index + 1, $count));
    }

    /**
     * @template T
     * @param callable(): T $work
     * @return T
     */
    private function locked(callable $work): mixed
    {
        if ((int) $this->db->value('SELECT GET_LOCK(?, 0)', [self::LOCK]) !== 1) {
            throw new ApiError(409, 'busy', 'A migration run is going on – try again in a moment.');
        }
        try {
            return $work();
        } finally {
            $this->db->value('SELECT RELEASE_LOCK(?)', [self::LOCK]);
        }
    }

    private function runner(): MigrationRunner
    {
        $dir = $this->settings->path('MIGRATIONS_PATH');
        return new MigrationRunner($this->db, $dir !== '' ? $dir : $this->settings->root() . '/migrations');
    }

    private function backupFile(string $id): string
    {
        $dir = $this->settings->path('MIGRATION_BACKUP_PATH');
        if ($dir === '') $dir = $this->settings->root() . '/storage/migration-backups';
        return $dir . '/' . gmdate('Y-m-d-His') . "-$id.jsonl";
    }

    private function relative(string $path): string
    {
        $root = str_replace('\\', '/', $this->settings->root()) . '/';
        $path = str_replace('\\', '/', $path);
        return str_starts_with($path, $root) ? substr($path, strlen($root)) : $path;
    }

    private function setStatus(string $id, string $status): void
    {
        if (!in_array($status, self::STATUSES, true)) throw new \InvalidArgumentException($status);
        $this->db->run('UPDATE dev_migration_runs SET status = ? WHERE id = ?', [$status, $id]);
    }

    /** @return array<string, mixed> */
    private function row(string $id): array
    {
        try {
            $row = $this->db->one('SELECT * FROM dev_migration_runs WHERE id = ?', [$id]);
        } catch (Throwable) {
            throw new ApiError(503, 'not_available', 'Migrations through the API are not set up yet (migration 2026-10-11-developer-role).');
        }
        return $row ?? throw new ApiError(404, 'not_found', 'No such migration run.');
    }

    /**
     * @param array<string, mixed> $row
     * @return array{files: list<array<string, mixed>>}
     */
    private function decode(array $row): array
    {
        $plan = json_decode((string) $row['plan'], true);
        return ['files' => is_array($plan) && is_array($plan['files'] ?? null) ? array_values($plan['files']) : []];
    }

    /**
     * @param array<string, mixed> $row
     * @return array<string, mixed>
     */
    private function summary(array $row): array
    {
        $files = $this->decode($row)['files'];
        return [
            'id' => (string) $row['id'],
            'status' => (string) $row['status'],
            'files' => array_map(fn(array $f) => (string) $f['name'], $files),
            'errors' => count(array_filter($files, fn(array $f) => ($f['error'] ?? null) !== null)),
            'created_via' => (string) $row['created_via'],
            'created_at' => (string) $row['created_at'],
            'applied_at' => $row['applied_at'] !== null ? (string) $row['applied_at'] : null,
            'error' => $row['error'] !== null ? (string) $row['error'] : null,
        ];
    }

    /**
     * @param array<string, mixed> $row
     * @return array<string, mixed>
     */
    private function full(array $row): array
    {
        return $this->summary($row) + [
            'plan' => $this->decode($row)['files'],
            'created_by' => $row['created_by'] !== null ? (string) $row['created_by'] : null,
            'applied_by' => $row['applied_by'] !== null ? (string) $row['applied_by'] : null,
            'log' => $row['log'] !== null ? (string) $row['log'] : null,
            'backup' => $row['backup'] !== null ? (string) $row['backup'] : null,
        ];
    }
}
