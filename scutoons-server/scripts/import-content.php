<?php
/**
 * Seeds the content store (content_versions, api/lib/Content.php) from the viewer's static config files
 * as published version 1 – the files the viewer loader reads from the store: toon.json, season<N>.json,
 * chapter<N>.json and the VN scene files chapter<N>/<name>.json, for every toon and locale folder.
 *
 *   php scripts/import-content.php <path to scutoons viewer/static/api> [options]
 *
 *   --toon=<id>        only this toon folder
 *   --db=<file>        use this SQLite file instead of the configured database (e.g. a copy for tests)
 *   --update           files that are already stored get a new published version when the static file differs
 *                      (default: stored files are skipped – the store is the source of truth once seeded)
 *   --dry-run          only report
 *   --sql              print idempotent MySQL INSERTs instead of writing anything (production has no shell: run the
 *                      output in phpMyAdmin after db/migrations/2026-09-30-content-versions.mysql.sql). Each statement
 *                      skips its file when the store already has a version of it.
 *
 * Idempotent: running it twice imports nothing the second time. The content_versions table is created when it
 * is missing (same statements as db/migrations/2026-09-30-content-versions.*.sql).
 * Production: `php scripts/import-content.php <path> --sql > content-seed.sql`, then import that file in phpMyAdmin.
 */
if (PHP_SAPI !== 'cli') {
    http_response_code(404);
    exit;
}

require_once __DIR__ . '/../api/lib/bootstrap.php';
require_once __DIR__ . '/../api/lib/Content.php';

$args = array_slice($argv, 1);
$root = null;
$options = ['toon' => null, 'db' => null, 'update' => false, 'dry-run' => false, 'sql' => false];
foreach ($args as $arg) {
    if (preg_match('/^--(toon|db)=(.+)$/', $arg, $m)) $options[$m[1]] = $m[2];
    elseif (in_array($arg, ['--update', '--dry-run', '--sql'], true)) $options[substr($arg, 2)] = true;
    elseif (!str_starts_with($arg, '--') && $root === null) $root = $arg;
    else {
        fwrite(STDERR, "Unknown argument $arg\n");
        exit(2);
    }
}
if ($root === null || !is_dir($root)) {
    fwrite(STDERR, "Usage: php scripts/import-content.php <path to viewer/static/api> [--toon=<id>] [--db=<sqlite file>] [--update] [--dry-run] [--sql]\n");
    exit(2);
}
$root = rtrim(str_replace('\\', '/', realpath($root)), '/');
$sqlMode = $options['sql'];

/** A MySQL string literal (backslash escapes, as in mysqldump) */
function sqlString(string $value): string
{
    return "'" . strtr($value, ['\\' => '\\\\', "'" => "\\'", "\0" => '\\0', "\n" => '\\n', "\r" => '\\r', "\x1a" => '\\Z']) . "'";
}

/** Report lines go to STDERR in --sql mode so the output stays a clean SQL file */
function report(string $line): void
{
    global $sqlMode;
    if ($sqlMode) fwrite(STDERR, $line);
    else echo $line;
}

if ($options['db'] !== null) {
    if (!is_file($options['db'])) {
        fwrite(STDERR, "No such database file: {$options['db']}\n");
        exit(2);
    }
    $pdo = new PDO('sqlite:' . $options['db'], null, null, [PDO::ATTR_ERRMODE => PDO::ERRMODE_EXCEPTION, PDO::ATTR_DEFAULT_FETCH_MODE => PDO::FETCH_ASSOC]);
    $pdo->exec('PRAGMA busy_timeout = 5000');
    // Db keeps its connection in a private static – point it at the given file
    (new ReflectionProperty(Db::class, 'pdo'))->setValue(null, $pdo);
}

if ($sqlMode) {
    echo "-- Content store seed from the viewer's static configs (scripts/import-content.php --sql, " . gmdate('Y-m-d H:i') . " UTC).\n";
    echo "-- Run db/migrations/2026-09-30-content-versions.mysql.sql first. Idempotent: files already in the store are skipped.\n";
    echo "SET NAMES utf8mb4;\n";
} else {
    // content_versions must exist
    $driver = Db::isSqlite() ? 'sqlite' : 'mysql';
    $hasTable = $driver === 'sqlite'
        ? Db::value("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'content_versions'") !== null
        : Db::value("SHOW TABLES LIKE 'content_versions'") !== null;
    if (!$hasTable) {
        $sql = preg_replace('/^\s*--.*$/m', '', (string) file_get_contents(__DIR__ . "/../db/migrations/2026-09-30-content-versions.$driver.sql"));
        foreach (array_filter(array_map('trim', preg_split('/;\s*(\r?\n|$)/', (string) $sql))) as $statement) {
            Db::pdo()->exec($statement);
        }
        echo "Created table content_versions\n";
    }
}

/** Store-managed files below <toon>/<locale>/ */
function isContentFile(string $relative): bool
{
    return (bool) preg_match('#^(toon|season\d+|chapter\d+)\.json$#', $relative)
        || (bool) preg_match('#^chapter\d+/[a-z0-9_-]+\.json$#', $relative);
}

$stats = ['imported' => 0, 'updated' => 0, 'skipped' => 0, 'unchanged' => 0, 'invalid' => 0];
$toons = array_filter(glob("$root/*", GLOB_ONLYDIR) ?: [], fn($d) => preg_match(Content::TOON_PATTERN, basename($d)));
foreach ($toons as $toonDir) {
    $toon = basename($toonDir);
    if ($options['toon'] !== null && $toon !== $options['toon']) continue;
    foreach (glob("$toonDir/*", GLOB_ONLYDIR) ?: [] as $localeDir) {
        $locale = basename($localeDir);
        if (!preg_match(Content::LOCALE_PATTERN, $locale)) continue;
        $files = array_merge(glob("$localeDir/*.json") ?: [], glob("$localeDir/*/*.json") ?: []);
        sort($files);
        foreach ($files as $file) {
            $relative = substr(str_replace('\\', '/', $file), strlen($localeDir) + 1);
            if (!isContentFile($relative)) continue;
            $label = "$toon/$locale/$relative";
            try {
                Content::checkKey($toon, $locale, $relative);
                $decoded = json_decode((string) file_get_contents($file));
                if ($decoded === null && json_last_error() !== JSON_ERROR_NONE) {
                    throw new ApiError(422, 'invalid_json', json_last_error_msg());
                }
                $json = Content::encode($decoded);
            } catch (ApiError $error) {
                report("  invalid   $label ({$error->getMessage()})\n");
                $stats['invalid']++;
                continue;
            }
            if ($sqlMode) {
                $where = '`toon` = ' . sqlString($toon) . ' AND `locale` = ' . sqlString($locale) . ' AND `path` = ' . sqlString($relative);
                echo "INSERT INTO `content_versions` (`toon`, `locale`, `path`, `json`, `author`, `created_at`, `status`, `message`)\n"
                    . 'SELECT ' . implode(', ', array_map('sqlString', [$toon, $locale, $relative, $json])) . ', NULL, '
                    . sqlString(Db::now()) . ", 'published', 'import from static files' FROM DUAL\n"
                    . "WHERE NOT EXISTS (SELECT 1 FROM `content_versions` WHERE $where);\n";
                report("  statement $label (" . strlen($json) . " bytes)\n");
                $stats['imported']++;
                continue;
            }
            $latest = Content::latest($toon, $locale, $relative);
            if ($latest && !$options['update']) {
                echo "  skipped   $label (stored, version {$latest['id']})\n";
                $stats['skipped']++;
                continue;
            }
            $published = $latest ? Content::published($toon, $locale, $relative) : null;
            if ($published && $published['json'] === $json) {
                echo "  unchanged $label\n";
                $stats['unchanged']++;
                continue;
            }
            $kind = $latest ? 'updated' : 'imported';
            echo sprintf("  %-9s %s (%d bytes)\n", $kind, $label, strlen($json));
            $stats[$kind]++;
            if ($options['dry-run']) continue;
            Db::transaction(function () use ($toon, $locale, $relative, $json, $latest) {
                $id = Content::insert($toon, $locale, $relative, $json, null, 'draft',
                    $latest ? 'update from static files' : 'import from static files');
                Content::setPublished(['id' => $id, 'toon' => $toon, 'locale' => $locale, 'path' => $relative]);
            });
        }
    }
}

report(sprintf("\n%s%d %s, %d updated, %d unchanged, %d skipped, %d invalid\n",
    $options['dry-run'] ? '[dry run] ' : '', $stats['imported'], $sqlMode ? 'statements' : 'imported',
    $stats['updated'], $stats['unchanged'], $stats['skipped'], $stats['invalid']));
exit($stats['invalid'] > 0 ? 1 : 0);
