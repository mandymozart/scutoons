<?php
declare(strict_types=1);

namespace Scutoons\Console;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Content\ContentService;
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

#[AsCommand(name: 'import-content', description: "Seed the content store from the viewer's static config files (published versions)")]
final class ImportContentCommand extends Command
{
    public function __construct(
        private readonly Database $db,
        private readonly Clock $clock,
        private readonly Settings $settings,
    ) {
        parent::__construct();
    }

    protected function configure(): void
    {
        $this->addArgument('root', InputArgument::REQUIRED, 'Path to the viewer\'s static/api folder')
            ->addOption('toon', null, InputOption::VALUE_REQUIRED, 'Only this toon folder')
            ->addOption('db', null, InputOption::VALUE_REQUIRED, 'Use this SQLite file instead of the configured database (e.g. a copy for tests)')
            ->addOption('update', null, InputOption::VALUE_NONE, 'Stored files get a new published version when the static file differs')
            ->addOption('dry-run', null, InputOption::VALUE_NONE, 'Only report')
            ->addOption('sql', null, InputOption::VALUE_NONE, 'Print idempotent MySQL INSERTs instead of writing (production seed for phpMyAdmin)')
            ->setHelp(<<<'HELP'
                Stores toon.json, season<N>.json, chapter<N>.json and the VN scene files chapter<N>/<name>.json of every
                toon and locale folder as published version 1 (author NULL, message "import from static files").
                Idempotent: files already in the store are skipped (the store is the source of truth once seeded).
                Production: bin/console import-content <path> --sql > content-seed.sql, then import that file in phpMyAdmin
                after migrations/mysql/2026-09-30-content-versions.mysql.sql. Each statement skips files already stored.
                HELP);
    }

    protected function execute(InputInterface $input, OutputInterface $output): int
    {
        $errors = $output instanceof ConsoleOutputInterface ? $output->getErrorOutput() : $output;
        $root = (string) $input->getArgument('root');
        if (!is_dir($root)) {
            $errors->writeln("Usage: bin/console import-content <path to viewer/static/api> [--toon=<id>] [--db=<sqlite file>] [--update] [--dry-run] [--sql]");
            return 2;
        }
        $root = rtrim(str_replace('\\', '/', (string) realpath($root)), '/');
        $sqlMode = (bool) $input->getOption('sql');
        $update = (bool) $input->getOption('update');
        $dryRun = (bool) $input->getOption('dry-run');
        $onlyToon = $input->getOption('toon');
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
        $content = new ContentService($db, $this->clock, $this->settings);

        if ($sqlMode) {
            $output->writeln("-- Content store seed from the viewer's static configs (bin/console import-content --sql, " . gmdate('Y-m-d H:i') . ' UTC).');
            $output->writeln('-- Run migrations/mysql/2026-09-30-content-versions.mysql.sql first. Idempotent: files already in the store are skipped.');
            $output->writeln('SET NAMES utf8mb4;');
        } else {
            // content_versions must exist
            $runner = new MigrationRunner($db, $this->settings->root() . '/migrations');
            if (!$runner->tableExists('content_versions')) {
                $runner->runFile($this->settings->root() . '/migrations/' . $db->driver() . '/2026-09-30-content-versions.' . $db->driver() . '.sql');
                $output->writeln('Created table content_versions');
            }
        }

        $stats = ['imported' => 0, 'updated' => 0, 'skipped' => 0, 'unchanged' => 0, 'invalid' => 0];
        $toons = array_filter(glob("$root/*", GLOB_ONLYDIR) ?: [], fn($d) => preg_match(ContentService::TOON_PATTERN, basename($d)) === 1);
        foreach ($toons as $toonDir) {
            $toon = basename($toonDir);
            if (is_string($onlyToon) && $toon !== $onlyToon) continue;
            foreach (glob("$toonDir/*", GLOB_ONLYDIR) ?: [] as $localeDir) {
                $locale = basename($localeDir);
                if (!preg_match(ContentService::LOCALE_PATTERN, $locale)) continue;
                $files = array_merge(glob("$localeDir/*.json") ?: [], glob("$localeDir/*/*.json") ?: []);
                sort($files);
                foreach ($files as $file) {
                    $relative = substr(str_replace('\\', '/', $file), strlen(str_replace('\\', '/', $localeDir)) + 1);
                    if (!self::isContentFile($relative)) continue;
                    $label = "$toon/$locale/$relative";
                    try {
                        ContentService::checkKey($toon, $locale, $relative);
                        $decoded = json_decode((string) file_get_contents($file));
                        if ($decoded === null && json_last_error() !== JSON_ERROR_NONE) {
                            throw new ApiError(422, 'invalid_json', json_last_error_msg());
                        }
                        $json = $content->encode($decoded);
                    } catch (ApiError $error) {
                        $report("  invalid   $label ({$error->getMessage()})\n");
                        $stats['invalid']++;
                        continue;
                    }
                    if ($sqlMode) {
                        $where = '`toon` = ' . self::sqlString($toon) . ' AND `locale` = ' . self::sqlString($locale) . ' AND `path` = ' . self::sqlString($relative);
                        $output->write("INSERT INTO `content_versions` (`toon`, `locale`, `path`, `json`, `author`, `created_at`, `status`, `message`)\n"
                            . 'SELECT ' . implode(', ', array_map([self::class, 'sqlString'], [$toon, $locale, $relative, $json])) . ', NULL, '
                            . self::sqlString($this->clock->utc()) . ", 'published', 'import from static files' FROM DUAL\n"
                            . "WHERE NOT EXISTS (SELECT 1 FROM `content_versions` WHERE $where);\n");
                        $report("  statement $label (" . strlen($json) . " bytes)\n");
                        $stats['imported']++;
                        continue;
                    }
                    $latest = $content->latest($toon, $locale, $relative);
                    if ($latest && !$update) {
                        $output->write("  skipped   $label (stored, version {$latest['id']})\n");
                        $stats['skipped']++;
                        continue;
                    }
                    $published = $latest ? $content->published($toon, $locale, $relative) : null;
                    if ($published && $published['json'] === $json) {
                        $output->write("  unchanged $label\n");
                        $stats['unchanged']++;
                        continue;
                    }
                    $kind = $latest ? 'updated' : 'imported';
                    $output->write(sprintf("  %-9s %s (%d bytes)\n", $kind, $label, strlen($json)));
                    $stats[$kind]++;
                    if ($dryRun) continue;
                    $db->transaction(function () use ($content, $toon, $locale, $relative, $json, $latest) {
                        $id = $content->insert($toon, $locale, $relative, $json, null, 'draft', $latest ? 'update from static files' : 'import from static files');
                        $content->setPublished(['id' => $id, 'toon' => $toon, 'locale' => $locale, 'path' => $relative]);
                    });
                }
            }
        }

        $report(sprintf("\n%s%d %s, %d updated, %d unchanged, %d skipped, %d invalid\n",
            $dryRun ? '[dry run] ' : '', $stats['imported'], $sqlMode ? 'statements' : 'imported',
            $stats['updated'], $stats['unchanged'], $stats['skipped'], $stats['invalid']));
        return $stats['invalid'] > 0 ? self::FAILURE : self::SUCCESS;
    }

    /** Store-managed files below <toon>/<locale>/ */
    private static function isContentFile(string $relative): bool
    {
        return preg_match('#^(toon|season\d+|chapter\d+)\.json$#', $relative) === 1
            || preg_match('#^chapter\d+/[a-z0-9_-]+\.json$#', $relative) === 1;
    }

    /** A MySQL string literal (backslash escapes, as in mysqldump) */
    public static function sqlString(string $value): string
    {
        return "'" . strtr($value, ['\\' => '\\\\', "'" => "\\'", "\0" => '\\0', "\n" => '\\n', "\r" => '\\r', "\x1a" => '\\Z']) . "'";
    }
}
