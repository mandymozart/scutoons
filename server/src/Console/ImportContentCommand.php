<?php
declare(strict_types=1);

namespace Scutoons\Console;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Content\ContentService;
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

#[AsCommand(name: 'import-content', description: "Seed the content store from the viewer's static config files (published versions)")]
final class ImportContentCommand extends Command
{
    /**
     * Toons that live in the static files for development and the creator only (the generated VN test bed
     * `viewer/static/api/sandbox`, shared/content/scripts/make-sandbox.ts): never part of the production seed (`--sql`)
     * unless asked for by name (`--toon=sandbox`).
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
        $this->addArgument('root', InputArgument::REQUIRED, 'Path to the viewer\'s static/api folder')
            ->addOption('toon', null, InputOption::VALUE_REQUIRED, 'Only this toon folder')
            ->addOption('db', null, InputOption::VALUE_REQUIRED, 'Use this SQLite file instead of the configured database (e.g. a copy for tests)')
            ->addOption('update', null, InputOption::VALUE_NONE, 'Stored files get a new published version when the static file differs')
            ->addOption('dry-run', null, InputOption::VALUE_NONE, 'Only report')
            ->addOption('sql', null, InputOption::VALUE_NONE, 'Print idempotent MySQL INSERTs instead of writing (production seed for phpMyAdmin)')
            ->addOption('only', null, InputOption::VALUE_REQUIRED, 'Only these file names, comma separated (e.g. characters.json,cast.json)')
            ->setHelp(<<<'HELP'
                Stores toon.json, season<N>.json, chapter<N>.json, the VN scene files chapter<N>/<name>.json and the map
                (locations.json, routes.json) of every toon and locale folder as published
                version 1 (author NULL, message "import from static files"); the root toons.json as one <toon>/_/index.json per
                entry and the root events.json as scutoons/_/events.json (agents/DATA.md); in the
                language-neutral folder `_`: credits.json, people.json, roles.json (the credits seed in shared/content/seed),
                characters.json + moods.json + asset-requests.json + cast.json (the characters seed; `--only=characters.json,moods.json,asset-requests.json,cast.json` imports just those).
                Idempotent: files already in the store are skipped (the store is the source of truth once seeded).
                Production: bin/console import-content <path> --sql > content-seed.sql, then import that file in phpMyAdmin
                after migrations/mysql/2026-09-30-content-versions.mysql.sql. Each statement skips files already stored.
                The dev-only sandbox toon (DEV_ONLY_TOONS) is left out of --sql unless named with --toon.
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
        $onlyFiles = is_string($input->getOption('only')) ? array_values(array_filter(array_map('trim', explode(',', $input->getOption('only'))))) : [];
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
        $devOnlyReported = [];
        foreach (self::sources($root) as [$toon, $locale, $relative, $file, $label, $read]) {
            if (!self::includesToon($toon, is_string($onlyToon) ? $onlyToon : null, $sqlMode)) {
                if (!is_string($onlyToon) && !isset($devOnlyReported[$toon])) {
                    $report("  skipped   $toon (dev only, not in the production seed; --toon=$toon to include it)\n");
                    $devOnlyReported[$toon] = true;
                }
                continue;
            }
            if (!self::matchesOnly($relative, $onlyFiles) && !self::matchesOnly(basename($file), $onlyFiles)) continue;
            try {
                ContentService::checkKey($toon, $locale, $relative);
                $decoded = $read();
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

        $report(sprintf("\n%s%d %s, %d updated, %d unchanged, %d skipped, %d invalid\n",
            $dryRun ? '[dry run] ' : '', $stats['imported'], $sqlMode ? 'statements' : 'imported',
            $stats['updated'], $stats['unchanged'], $stats['skipped'], $stats['invalid']));
        return $stats['invalid'] > 0 ? self::FAILURE : self::SUCCESS;
    }

    /**
     * Every document below the root, in import order: the root files first (`toons.json` → one `<toon>/_/index.json`
     * per entry, in the file's order = the toons list's order; `events.json` → `scutoons/_/events.json`), then every toon
     * and locale folder. Each: [scope, locale, path, source file, label, reader (decoded JSON or ApiError)].
     * @return list<array{0: string, 1: string, 2: string, 3: string, 4: string, 5: callable(): mixed}>
     */
    public static function sources(string $root): array
    {
        $sources = [];
        if (is_file("$root/toons.json")) {
            $index = json_decode((string) file_get_contents("$root/toons.json"));
            foreach (is_array($index) ? $index : [] as $entry) {
                $id = is_object($entry) && is_string($entry->id ?? null) ? $entry->id : null;
                if ($id === null || !preg_match(ContentService::TOON_PATTERN, $id)) continue;
                $sources[] = [$id, ContentService::NEUTRAL_LOCALE, 'index.json', "$root/toons.json", "$id/_/index.json (toons.json)", fn() => $entry];
            }
        }
        if (is_file("$root/events.json")) {
            $sources[] = [Library::PLATFORM, ContentService::NEUTRAL_LOCALE, 'events.json', "$root/events.json", Library::PLATFORM . '/_/events.json (events.json)', fn() => self::readJson("$root/events.json")];
        }
        $toons = array_filter(glob("$root/*", GLOB_ONLYDIR) ?: [], fn($d) => preg_match(ContentService::TOON_PATTERN, basename($d)) === 1);
        foreach ($toons as $toonDir) {
            $toon = basename($toonDir);
            foreach (glob("$toonDir/*", GLOB_ONLYDIR) ?: [] as $localeDir) {
                $locale = basename($localeDir);
                if (!preg_match(ContentService::LOCALE_PATTERN, $locale)) continue;
                $files = array_merge(glob("$localeDir/*.json") ?: [], glob("$localeDir/*/*.json") ?: []);
                sort($files);
                foreach ($files as $file) {
                    $relative = substr(str_replace('\\', '/', $file), strlen(str_replace('\\', '/', $localeDir)) + 1);
                    if (!self::isContentFile($locale, $relative)) continue;
                    $sources[] = [$toon, $locale, $relative, $file, "$toon/$locale/$relative", fn() => self::readJson($file)];
                }
            }
        }
        return $sources;
    }

    /** A JSON file decoded with objects (keeps `{}`), or an ApiError */
    public static function readJson(string $file): mixed
    {
        $decoded = json_decode((string) file_get_contents($file));
        if ($decoded === null && json_last_error() !== JSON_ERROR_NONE) {
            throw new ApiError(422, 'invalid_json', json_last_error_msg());
        }
        return $decoded;
    }

    /**
     * Store-managed files below <toon>/<locale>/. The language-neutral locale `_` holds the credits files:
     * `<toon>/_/credits.json` and, in the pseudo-toon `scutoons`, `people.json` + `roles.json`
     * (seed: shared/content/seed, written by shared/content/scripts/migrate-credits.ts); the characters registry
     * `scutoons/_/characters.json`, the moods / asset requests `scutoons/_/{moods,asset-requests}.json` and the casts `<toon>/_/cast.json` (shared/content/scripts/migrate-characters.ts).
     */
    public static function isContentFile(string $locale, string $relative): bool
    {
        if ($locale === ContentService::NEUTRAL_LOCALE) return preg_match('#^(credits|people|roles|characters|moods|asset-requests|cast)\.json$#', $relative) === 1;
        return preg_match('#^(toon|season\d+|chapter\d+|locations|routes)\.json$#', $relative) === 1
            || preg_match('#^chapter\d+/[a-z0-9_-]+\.json$#', $relative) === 1;
    }

    /** Whether a toon folder is imported: dev-only toons stay out of the production seed (`--sql`) unless named with `--toon`. */
    public static function includesToon(string $toon, ?string $onlyToon, bool $sqlMode): bool
    {
        if ($onlyToon !== null) return $toon === $onlyToon;
        return !($sqlMode && in_array($toon, self::DEV_ONLY_TOONS, true));
    }

    /**
     * `--only` filter: no names = every file; else the file name (last path segment) must be listed
     * @param list<string> $only
     */
    public static function matchesOnly(string $relative, array $only): bool
    {
        return $only === [] || in_array(basename($relative), $only, true);
    }

    /** A MySQL string literal (backslash escapes, as in mysqldump) */
    public static function sqlString(string $value): string
    {
        return "'" . strtr($value, ['\\' => '\\\\', "'" => "\\'", "\0" => '\\0', "\n" => '\\n', "\r" => '\\r', "\x1a" => '\\Z']) . "'";
    }
}
