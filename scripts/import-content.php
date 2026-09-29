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
 *
 * Idempotent: running it twice imports nothing the second time. The content_versions table is created when it
 * is missing (same statements as db/migrations/2026-09-30-content-versions.*.sql).
 * Production (no shell): run it locally against a copy of the production data or use phpMyAdmin with --dry-run's list.
 */
if (PHP_SAPI !== 'cli') {
    http_response_code(404);
    exit;
}

require_once __DIR__ . '/../api/lib/bootstrap.php';
require_once __DIR__ . '/../api/lib/Content.php';

$args = array_slice($argv, 1);
$root = null;
$options = ['toon' => null, 'db' => null, 'update' => false, 'dry-run' => false];
foreach ($args as $arg) {
    if (preg_match('/^--(toon|db)=(.+)$/', $arg, $m)) $options[$m[1]] = $m[2];
    elseif ($arg === '--update' || $arg === '--dry-run') $options[substr($arg, 2)] = true;
    elseif (!str_starts_with($arg, '--') && $root === null) $root = $arg;
    else {
        fwrite(STDERR, "Unknown argument $arg\n");
        exit(2);
    }
}
if ($root === null || !is_dir($root)) {
    fwrite(STDERR, "Usage: php scripts/import-content.php <path to viewer/static/api> [--toon=<id>] [--db=<sqlite file>] [--update] [--dry-run]\n");
    exit(2);
}
$root = rtrim(str_replace('\\', '/', realpath($root)), '/');

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
                echo "  invalid   $label ({$error->getMessage()})\n";
                $stats['invalid']++;
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

echo sprintf("\n%s%d imported, %d updated, %d unchanged, %d skipped, %d invalid\n",
    $options['dry-run'] ? '[dry run] ' : '', $stats['imported'], $stats['updated'], $stats['unchanged'], $stats['skipped'], $stats['invalid']);
exit($stats['invalid'] > 0 ? 1 : 0);
