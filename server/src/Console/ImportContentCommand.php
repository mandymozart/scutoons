<?php
declare(strict_types=1);

namespace Scutoons\Console;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Content\EntityKey;
use Scutoons\Domain\Content\CollectionDocs;
use Scutoons\Domain\Content\EntityStore;
use Scutoons\Domain\Library\Library;
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
     * shared/content/scripts/make-sandbox.ts): never part of the production seed (`--sql`) unless named with `--toon`,
     * never in the public toons list (Library).
     */
    public const DEV_ONLY_TOONS = Library::DEV_ONLY_TOONS;

    public function __construct(
        private readonly Database $db,
        private readonly Clock $clock,
        private readonly Settings $settings,
    ) {
        parent::__construct();
    }

    protected function configure(): void
    {
        $this->addArgument('root', InputArgument::REQUIRED, 'A folder in the interchange layout (an export-content folder, the test fixture server/tests/fixtures/content)')
            ->addOption('toon', null, InputOption::VALUE_REQUIRED, 'Only this toon')
            ->addOption('update', null, InputOption::VALUE_NONE, 'Stored entities get a new published version when the file differs')
            ->addOption('dry-run', null, InputOption::VALUE_NONE, 'Only report')
            ->addOption('sql', null, InputOption::VALUE_NONE, 'Print idempotent MySQL INSERTs instead of writing (production seed for phpMyAdmin)')
            ->addOption('only', null, InputOption::VALUE_REQUIRED, 'Only these entity kinds, comma separated (e.g. characters,moods,asset-requests,cast)')
            ->setHelp(<<<'HELP'
                Reads the interchange layout (agents/DATA.md §4, Console\ContentFiles: toons.json, events.json,
                <toon>/<locale>/{toon,season<N>,chapter<N>,locations,routes,script}.json,
                <toon>/_/{cast,credits}.json, scutoons/_/{characters,moods,asset-requests,people,roles,map,colorations}.json) and stores every
                entity as a published version (author NULL, message "import"), keyed by its entity key.
                Idempotent: entities already in the store are skipped (the store is the source of truth once seeded);
                --update adds a new published version where the file differs.
                --sql prints the same as guarded INSERTs (each skips stored entities) for phpMyAdmin. Production is the source of
                content (the creator); a local database gets it from db:prod-seed.
                The dev-only sandbox toon (DEV_ONLY_TOONS) is left out of --sql unless named with --toon; the creator-only
                screenplays (script:<toon>:<l>, EntityKey::CREATOR_ONLY_KINDS) unless named with --only=script.
                HELP);
    }

    protected function execute(InputInterface $input, OutputInterface $output): int
    {
        $errors = $output instanceof ConsoleOutputInterface ? $output->getErrorOutput() : $output;
        $root = (string) $input->getArgument('root');
        if (!is_dir($root)) {
            $errors->writeln('Usage: bin/console import-content <folder> [--toon=<id>] [--update] [--dry-run] [--sql] [--only=<kinds>]');
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
            // `--only=map` / `--only=characters`: with their entries (one file, many documents – agents/MAP-DOCUMENTS.md)
            if ($onlyKinds !== [] && !in_array(CollectionDocs::ofPart($key->kind)['key'] ?? $key->kind, $onlyKinds, true) && !in_array($key->kind, $onlyKinds, true)) continue;
            if (!self::includesKind($key->kind, $onlyKinds, $sqlMode)) {
                if (!isset($devOnlyReported["kind:$key->kind"])) {
                    $report("  skipped   {$key->kind} (creator only, not in the production seed; --only={$key->kind} to include it)\n");
                    $devOnlyReported["kind:$key->kind"] = true;
                }
                continue;
            }
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

    /**
     * Whether an entity kind is imported: creator-only kinds (EntityKey::CREATOR_ONLY_KINDS, the screenplays) stay out
     * of the production seed (`--sql`) unless `--only` names them; a local import takes them like everything else.
     * @param list<string> $onlyKinds
     */
    public static function includesKind(string $kind, array $onlyKinds, bool $sqlMode): bool
    {
        if (in_array($kind, $onlyKinds, true)) return true;
        return !($sqlMode && in_array($kind, EntityKey::CREATOR_ONLY_KINDS, true));
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
