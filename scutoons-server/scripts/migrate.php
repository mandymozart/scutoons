<?php
/**
 * Applies the dated migrations in db/migrations/ that match the database driver
 * (`*.sqlite.sql` for SQLite, `*.mysql.sql` for MySQL/MariaDB) and records them in `schema_migrations`.
 * `ALTER TABLE x ADD COLUMN y` is skipped when the column exists, so re-running is safe.
 * Legacy files without a driver suffix (2025-10-19, 2026-06-16) are MySQL-only and already applied in production.
 *
 *   php scripts/migrate.php            apply pending migrations to the configured database (config.local.php)
 *   php scripts/migrate.php --status   list migrations and whether they are applied
 *
 * Production (shared host, no shell): run the *.mysql.sql files in phpMyAdmin instead – they are idempotent.
 */
if (PHP_SAPI !== 'cli') {
    http_response_code(404);
    exit;
}

require_once __DIR__ . '/../db.php';

$pdo = getDbConnection();
$driver = $pdo->getAttribute(PDO::ATTR_DRIVER_NAME) === 'sqlite' ? 'sqlite' : 'mysql';
$statusOnly = in_array('--status', $argv, true);

$pdo->exec($driver === 'sqlite'
    ? 'CREATE TABLE IF NOT EXISTS schema_migrations (name TEXT NOT NULL PRIMARY KEY, applied_at TEXT NOT NULL)'
    : 'CREATE TABLE IF NOT EXISTS `schema_migrations` (`name` VARCHAR(191) NOT NULL, `applied_at` DATETIME NOT NULL, PRIMARY KEY (`name`)) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci');

$applied = array_flip($pdo->query('SELECT name FROM schema_migrations')->fetchAll(PDO::FETCH_COLUMN));
$files = glob(__DIR__ . "/../db/migrations/*.$driver.sql");
sort($files);

function hasColumn(PDO $pdo, string $driver, string $table, string $column): bool
{
    $rows = $driver === 'sqlite'
        ? $pdo->query("PRAGMA table_info($table)")->fetchAll()
        : $pdo->query("SHOW COLUMNS FROM `$table`")->fetchAll();
    foreach ($rows as $row) {
        if (strcasecmp((string) ($row['name'] ?? $row['Field'] ?? ''), $column) === 0) return true;
    }
    return false;
}

echo "Database driver: $driver\n";
foreach ($files as $file) {
    $name = basename($file);
    if (isset($applied[$name])) {
        echo "  applied  $name\n";
        continue;
    }
    if ($statusOnly) {
        echo "  pending  $name\n";
        continue;
    }
    $sql = preg_replace('/^\s*--.*$/m', '', (string) file_get_contents($file));
    $statements = array_filter(array_map('trim', preg_split('/;\s*(\r?\n|$)/', (string) $sql)));
    $skipped = 0;
    foreach ($statements as $statement) {
        if (preg_match('/^ALTER\s+TABLE\s+`?(\w+)`?\s+ADD\s+COLUMN\s+(?:IF\s+NOT\s+EXISTS\s+)?`?(\w+)`?/i', $statement, $m)
            && hasColumn($pdo, $driver, $m[1], $m[2])) {
            $skipped++;
            continue;
        }
        $pdo->exec($statement);
    }
    $stmt = $pdo->prepare('INSERT INTO schema_migrations (name, applied_at) VALUES (?, ?)');
    $stmt->execute([$name, gmdate('Y-m-d H:i:s')]);
    echo "  applied  $name (" . count($statements) . " statements, $skipped skipped)\n";
}
