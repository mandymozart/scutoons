<?php
declare(strict_types=1);

namespace Scutoons\Console;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Content\EntityKey;
use Scutoons\Domain\Content\EntityStore;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Infrastructure\Database\MigrationRunner;
use Symfony\Component\Console\Attribute\AsCommand;
use Symfony\Component\Console\Command\Command;
use Symfony\Component\Console\Input\InputArgument;
use Symfony\Component\Console\Input\InputInterface;
use Symfony\Component\Console\Input\InputOption;
use Symfony\Component\Console\Output\ConsoleOutputInterface;
use Symfony\Component\Console\Output\OutputInterface;

#[AsCommand(name: 'import-content', description: 'Import content from the interchange format (JSON folder layout) into the store as published versions')]
final class ImportContentCommand extends Command
{
    /**
     * Toons for development and the creator only (the generated VN test bed `sandbox`,
     * shared/content/scripts/make-sandbox.ts): never part of the production seed (`--sql`) unless named with `--toon`.
     */
    public const DEV_ONLY_TOONS = ['sandbox'];

    public function __construct(
        private readonly Database $db,
        private readonly Clock $clock,
        private readonly Settings $settings,
    ) {
        parent::__construct();
    }

    protected function configure(): void
    {
        $this->addArgument('root', InputArgument::REQUIRED, 'A folder in the interchange layout (e.g. shared/content/seed, an export-content folder)')
            ->addOption('toon', null, InputOption::VALUE_REQUIRED, 'Only this toon')
            ->addOption('db', null, InputOption::VALUE_REQUIRED, 'Use this SQLite file instead of the configured database (e.g. a copy for tests)')
            ->addOption('update', null, InputOption::VALUE_NONE, 'Stored entities get a new published version when the file differs')
            ->addOption('dry-run', null, InputOption::VALUE_NONE, 'Only report')
            ->addOption('sql', null, InputOption::VALUE_NONE, 'Print idempotent MySQL INSERTs instead of writing (production seed for phpMyAdmin)')
            ->addOption('only', null, InputOption::VALUE_REQUIRED, 'Only these entity kinds, comma separated (e.g. characters,moods,asset-requests,cast)')
            ->setHelp(<<<'HELP'
                Reads the interchange layout (agents/DATA.md §4, Console\ContentFiles: toons.json, events.json,
                <toon>/<locale>/{toon,season<N>,chapter<N>,locations,routes}.json, <toon>/<locale>/chapter<N>/<scene>.json,
                <toon>/_/{cast,credits}.json, scutoons/_/{characters,moods,asset-requests,people,roles}.json) and stores every
                entity as a published version (author NULL, message "import"), keyed by its entity key.
                Idempotent: entities already in the store are skipped (the store is the source of truth once seeded);
                --update adds a new published version where the file differs.
                Production: bin/console import-content shared/content/seed --sql > content-seed.sql, then import that file in
                phpMyAdmin after migrations/mysql/2026-09-30-entity-keys.mysql.sql. Each statement skips stored entities.
                The dev-only sandbox toon (DEV_ONLY_TOONS) is left out of --sql unless named with --toon.
                HELP);
    }

    protected function execute(InputInterface $input, OutputInterface $output): int
    {
        $errors = $output instanceof ConsoleOutputInterface ? $output->getErrorOutput() : $output;
        $root = (string) $input->getArgument('root');
        if (!is_dir($root)) {
            $errors->writeln('Usage: bin/console import-content <folder> [--toon=<id>] [--db=<sqlite file>] [--update] [--dry-run] [--sql] [--only=<kinds>]');
            return 2;
        }
        $root = rtrim(str_replace('\\', '/', (string) realpath($root)), '/');
        $sqlMode = (bool) $input->getOption('sql');
        $update = (bool) $input->getOption('update');
        $dryRun = (bool) $input->getOption('dry-run');
        $onlyToon = is_string($input->getOption('toon')) ? $input->getOption('toon') : null;
        $onlyKinds = is_string($input->getOption('only')) ? array_values(array_filter(array_map('trim', explode(',', $input->getOption('only'))))) : [];
        // Report lines go to STDERR in --sql mode so the output stays a clean SQL file
        $report = fn(string $line) => $sqlMode ? $errors->write($line) : $output->write($line);

        $db = $this->db;
        $dbFile = $input->getOption('db');
        if (is_string($dbFile)) {
            if (!is_file($dbFile)) {
                $errors->writeln("No such database file: $dbFile");
                return 2;
            }
            $db = Database::sqlite($dbFile);
        }
        $store = new EntityStore($db, $this->clock, $this->settings);

        if ($sqlMode) {
            $output->writeln('-- Content seed (bin/console import-content --sql, ' . gmdate('Y-m-d H:i') . ' UTC).');
            $output->writeln('-- Run migrations/mysql/2026-09-30-entity-keys.mysql.sql first. Idempotent: entities already stored are skipped.');
            $output->writeln('SET NAMES utf8mb4;');
        } else {
            $runner = new MigrationRunner($db, $this->settings->root() . '/migrations');
            if (!$runner->tableExists('entity_versions')) {
                $runner->migrate(fn(string $line) => $output->writeln($line));
            }
        }

        $stats = ['imported' => 0, 'updated' => 0, 'skipped' => 0, 'unchanged' => 0, 'invalid' => 0];
        $devOnlyReported = [];
        foreach (ContentFiles::sources($root) as [$keyString, $file, $label, $read]) {
            $key = EntityKey::parse($keyString);
            $toon = $key->toon;
            if (!self::includesToon($toon, $onlyToon, $sqlMode)) {
                if ($onlyToon === null && $toon !== null && !isset($devOnlyReported[$toon])) {
                    $report("  skipped   $toon (dev only, not in the production seed; --toon=$toon to include it)\n");
                    $devOnlyReported[$toon] = true;
                }
                continue;
            }
            if ($onlyKinds !== [] && !in_array($key->kind, $onlyKinds, true)) continue;
            try {
                $json = $store->encode($read());
            } catch (ApiError $error) {
                $report("  invalid   $label ({$error->getMessage()})\n");
                $stats['invalid']++;
                continue;
            }
            if ($sqlMode) {
                $output->write("INSERT INTO `entity_versions` (`entity_key`, `kind`, `toon`, `locale`, `json`, `author`, `created_at`, `status`, `message`)\n"
                    . 'SELECT ' . implode(', ', [self::sqlString($keyString), self::sqlString($key->kind), self::sqlNullable($key->toon), self::sqlNullable($key->locale), self::sqlString($json)])
                    . ', NULL, ' . self::sqlString($this->clock->utc()) . ", 'published', 'import' FROM DUAL\n"
                    . 'WHERE NOT EXISTS (SELECT 1 FROM `entity_versions` WHERE `entity_key` = ' . self::sqlString($keyString) . ");\n");
                $report("  statement $label (" . strlen($json) . " bytes)\n");
                $stats['imported']++;
                continue;
            }
            $latest = $store->latest($keyString);
            if ($latest && !$update) {
                $output->write("  skipped   $label (stored, version {$latest['id']})\n");
                $stats['skipped']++;
                continue;
            }
            $published = $latest ? $store->published($keyString) : null;
            if ($published && $published['json'] === $json) {
                $output->write("  unchanged $label\n");
                $stats['unchanged']++;
                continue;
            }
            $kind = $latest ? 'updated' : 'imported';
            $output->write(sprintf("  %-9s %s (%d bytes)\n", $kind, $label, strlen($json)));
            $stats[$kind]++;
            if (!$dryRun) $store->insertPublished($key, $json, $latest ? 'import (update)' : 'import');
        }

        $report(sprintf("\n%s%d %s, %d updated, %d unchanged, %d skipped, %d invalid\n",
            $dryRun ? '[dry run] ' : '', $stats['imported'], $sqlMode ? 'statements' : 'imported',
            $stats['updated'], $stats['unchanged'], $stats['skipped'], $stats['invalid']));
        return $stats['invalid'] > 0 ? self::FAILURE : self::SUCCESS;
    }

    /** Whether an entity's toon is imported: dev-only toons stay out of the production seed (`--sql`) unless named with `--toon` */
    public static function includesToon(?string $toon, ?string $onlyToon, bool $sqlMode): bool
    {
        if ($onlyToon !== null) return $toon === $onlyToon;
        return !($sqlMode && $toon !== null && in_array($toon, self::DEV_ONLY_TOONS, true));
    }

    /** A MySQL string literal (backslash escapes, as in mysqldump) */
    public static function sqlString(string $value): string
    {
        return "'" . strtr($value, ['\\' => '\\\\', "'" => "\\'", "\0" => '\\0', "\n" => '\\n', "\r" => '\\r', "\x1a" => '\\Z']) . "'";
    }

    private static function sqlNullable(?string $value): string
    {
        return $value === null ? 'NULL' : self::sqlString($value);
    }
}
