<?php
/**
 * Runs every HTTP contract suite against freshly started dev servers on a COPY of the local SQLite database
 * (composer test:contract; part of composer test). Nothing touches the configured database.
 *
 *   php tests/run-contract.php [--port=9180] [--keep]
 *
 * 1. copies DB_SQLITE_PATH (config.local.php) to a temp file, migrates the copy, drops its stored runtime settings and
 *    adds an expired and a revoked code to it
 * 2. starts the API `php -S 127.0.0.1:<port> -t public public/router.php` (QR_DB_SQLITE_PATH = the copy, a random
 *    QR_PROXY_SECRET) and the qr app (../qr) in front of it: <port+1> full API proxied, <port+2> PROXY_FULL_API=0,
 *    <port+3> with its API down (fallback page), <port+4> in front of a second API (<port+5>) with lookup + scan limit 1;
 *    <port+6> = an API on a FRESH database seeded with import-content from shared/content/seed (the interchange files)
 *    (CONTENT_SHOW_DRAFTS=1) for the library compare
 * 3. runs api-smoke, content-smoke, claim-race (claim + signup), admin-smoke, entities-smoke against the API,
 *    entities-compare against <port+6> and the printed-URL suite (../qr/tests/public-urls.php) against the qr app
 * 4. stops the servers and deletes the copy and the qr caches (--keep keeps the copy and prints its path)
 * Needs config.local.php with MAIL_TRANSPORT=log (the suites read one-time codes from the mail log).
 */
if (PHP_SAPI !== 'cli') {
    http_response_code(404);
    exit;
}

require __DIR__ . '/../vendor/autoload.php';

$root = dirname(__DIR__);
$port = 9180;
$keep = false;
foreach (array_slice($argv, 1) as $arg) {
    if (str_starts_with($arg, '--port=')) $port = (int) substr($arg, 7);
    elseif ($arg === '--keep') $keep = true;
}

$settings = Scutoons\Infrastructure\Config\Settings::load();
$source = $settings->path('DB_SQLITE_PATH');
if ($source === '' || !is_file($source)) {
    fwrite(STDERR, "The contract suites need a local SQLite database (DB_SQLITE_PATH in config.local.php).\n");
    exit(2);
}
if ($settings->get('MAIL_TRANSPORT') !== 'log') {
    fwrite(STDERR, "The contract suites need MAIL_TRANSPORT=log (they read the one-time codes from the mail log).\n");
    exit(2);
}
$copy = str_replace('\\', '/', sys_get_temp_dir()) . '/scutoons-contract-' . getmypid() . '.sqlite';
copy($source, $copy);
echo "Database copy: $copy (from $source)\n";

$copyDb = Scutoons\Infrastructure\Database\Database::sqlite($copy);
(new Scutoons\Infrastructure\Database\MigrationRunner($copyDb, $root . '/migrations'))->migrate(static fn() => null);
$copyDb->close();
$copyDb = null;

// The printed-URL suite needs an expired and a revoked code (the local database may have none): only in the copy
$pdo = new PDO('sqlite:' . $copy);
$insert = $pdo->prepare("INSERT OR IGNORE INTO qr (id, code, redirect_to, type, email, created_at, expires_at, revoked_at, revoked_reason) VALUES (?, ?, 'https://scutoons.com/catpope', 'membership', '', ?, ?, ?, ?)");
$insert->execute(['contract-expired', 'contractExpired1', '2020-01-01 00:00:00', '2020-01-02 00:00:00', null, null]);
$insert->execute(['contract-revoked', 'contractRevoked1', '2020-01-01 00:00:00', null, '2020-01-03 00:00:00', 'contract suite']);
// Stored runtime settings (admin app → Settings) would override the limits below: the copy starts with the defaults
$pdo->exec('DELETE FROM settings');
$pdo = null;

// A FRESH database seeded from the interchange files (shared/content/seed) only: the library compare (tests/entities-compare.php)
$fresh = str_replace('\\', '/', sys_get_temp_dir()) . '/scutoons-contract-fresh-' . getmypid() . '.sqlite';
@unlink($fresh);
$freshDb = Scutoons\Infrastructure\Database\Database::sqlite($fresh);
(new Scutoons\Infrastructure\Database\MigrationRunner($freshDb, $root . '/migrations'))->migrate(static fn() => null);
$freshDb->close();
$freshDb = null;
$contentSeed = dirname($root) . '/shared/content/seed';
foreach ([$contentSeed] as $folder) {
    exec(escapeshellarg(PHP_BINARY) . ' ' . escapeshellarg("$root/bin/console") . ' import-content ' . escapeshellarg($folder) . ' --db=' . escapeshellarg($fresh), $importOut, $importCode);
    if ($importCode !== 0) {
        fwrite(STDERR, "import-content $folder failed:\n" . implode("\n", array_slice($importOut, -10)) . "\n");
        exit(2);
    }
}

$qrRoot = dirname($root) . '/qr';
$tmp = str_replace('\\', '/', sys_get_temp_dir()) . '/scutoons-contract-' . getmypid();
$secret = bin2hex(random_bytes(16));
$env = getenv() + [];
$env['QR_DB_SQLITE_PATH'] = $copy;
$env['QR_QR_PROXY_SECRET'] = $secret;          // the API trusts the qr app's forwarded client IP / device id
$env['QR_ACCESS_MAX_PER_IP_HOUR'] = '100000';  // every suite runs from 127.0.0.1
$env['QR_SCAN_MAX_PER_IP_HOUR'] = '100000';    // (admin-smoke sets the scan limit to 3 and back through the admin API)
$env['QR_CONTENT_SHOW_DRAFTS'] = '0';          // the library gates as in production (entities-smoke)
$env['QR_CACHE_PATH'] = "$tmp-libcache";        // library answers of the test servers stay out of storage/cache
$servers = [];
$start = function (int $port, array $extraEnv, string $dir = '', string $ready = '/api/health/') use ($root, $env, &$servers): void {
    $dir = $dir !== '' ? $dir : $root;
    $log = str_replace('\\', '/', sys_get_temp_dir()) . "/scutoons-contract-server-$port.log";
    $proc = proc_open([PHP_BINARY, '-S', "127.0.0.1:$port", '-t', "$dir/public", "$dir/public/router.php"],
        [0 => ['file', 'php://stdin', 'r'], 1 => ['file', $log, 'w'], 2 => ['file', $log, 'a']], $pipes, $dir, $extraEnv + $env);
    if (!is_resource($proc)) throw new RuntimeException("could not start the server on :$port");
    $servers[] = $proc;
    for ($i = 0; $i < 100; $i++) {
        $ok = @file_get_contents("http://127.0.0.1:$port$ready");
        if ($ok !== false) return;
        usleep(100000);
    }
    throw new RuntimeException("server on :$port did not answer (log: $log)");
};
$qr = fn(int $apiPort, string $cache, array $extra = []): array => $extra + [
    'QRAPP_API_URL' => "http://127.0.0.1:$apiPort", 'QRAPP_QR_PROXY_SECRET' => $secret, 'QRAPP_CACHE_DIR' => "$tmp-$cache",
    'QRAPP_PROXY_FULL_API' => '1', 'QRAPP_WWW_URL' => 'http://viewer.test',
];

$results = [];
$exit = 0;
try {
    // API :port (+ a copy with a tiny lookup limit on :port+5); the qr app on :port+1 (full API proxied), :port+2
    // (PROXY_FULL_API=0), :port+3 (its API is down, same cache as :port+1), :port+4 (in front of the limited API)
    $start($port, []);
    $start($port + 5, ['QR_ACCESS_MAX_PER_IP_HOUR' => '1', 'QR_SCAN_MAX_PER_IP_HOUR' => '1']);
    $start($port + 6, ['QR_DB_SQLITE_PATH' => $fresh, 'QR_CONTENT_SHOW_DRAFTS' => '1']);
    $start($port + 1, $qr($port, 'cache'), $qrRoot, '/version.json');
    $start($port + 2, $qr($port, 'cache-off', ['QRAPP_PROXY_FULL_API' => '0']), $qrRoot, '/version.json');
    $start($port + 3, $qr(1, 'cache'), $qrRoot, '/version.json');
    $start($port + 4, $qr($port + 5, 'cache-limited'), $qrRoot, '/version.json');
    $base = "http://127.0.0.1:$port";
    $publicUrls = "$qrRoot/tests/public-urls.php";
    $suites = [
        'api-smoke' => ['tests/api-smoke.php', $base, 'http://127.0.0.1:' . ($port + 1)],
        'content-smoke' => ['tests/content-smoke.php', $base, $contentSeed],
        'claim-race (claim)' => ['tests/claim-race.php', '8', 'claim'],
        'claim-race (signup)' => ['tests/claim-race.php', '8', 'signup'],
        'admin-smoke' => ['tests/admin-smoke.php', $base, 'http://127.0.0.1:' . ($port + 1)],
        'entities-smoke' => ['tests/entities-smoke.php', $base],
        'entities-compare (fresh DB = the seed files)' => ['tests/entities-compare.php', 'http://127.0.0.1:' . ($port + 6), "--static=$contentSeed", "--seed=$contentSeed"],
        'public-urls (qr app → API)' => [$publicUrls, 'http://127.0.0.1:' . ($port + 1), "--api=$base", "--db=$copy",
            '--down=http://127.0.0.1:' . ($port + 3), '--limited=http://127.0.0.1:' . ($port + 4)],
        'public-urls (PROXY_FULL_API=0)' => [$publicUrls, 'http://127.0.0.1:' . ($port + 2), "--api=$base", "--db=$copy", '--full-api=off'],
    ];
    foreach ($suites as $name => $args) {
        echo "\n━━ $name ━━\n";
        $proc = proc_open(array_merge([PHP_BINARY], $args), [1 => ['pipe', 'w'], 2 => ['pipe', 'w']], $pipes, $root, $env);
        $output = stream_get_contents($pipes[1]) . stream_get_contents($pipes[2]);
        $code = proc_close($proc);
        $failed = array_filter(explode("\n", (string) $output), fn($line) => str_contains($line, 'FAIL'));
        echo $failed ? implode("\n", $failed) . "\n" : '';
        $lines = array_values(array_filter(array_map('trim', explode("\n", (string) $output))));
        $summary = end($lines) ?: '';
        echo ($code === 0 ? 'OK    ' : 'FAIL  ') . $summary . "\n";
        $results[$name] = [$code, $summary];
        if ($code !== 0) $exit = 1;
    }
} catch (Throwable $error) {
    fwrite(STDERR, $error->getMessage() . "\n");
    $exit = 2;
} finally {
    foreach ($servers as $proc) {
        proc_terminate($proc);
        proc_close($proc);
    }
    foreach (glob("$tmp-libcache/library/*") ?: [] as $file) @unlink($file);
    @rmdir("$tmp-libcache/library");
    @rmdir("$tmp-libcache");
    @unlink($fresh);
    foreach (glob("$tmp-cache*/*.json") ?: [] as $file) @unlink($file);
    foreach (glob("$tmp-cache*", GLOB_ONLYDIR) ?: [] as $dir) @rmdir($dir);
    if ($keep) echo "\nKept the database copy: $copy\n";
    else @unlink($copy);
}

echo "\nContract suites\n";
foreach ($results as $name => [$code, $summary]) echo sprintf("  %-4s %-44s %s\n", $code === 0 ? 'OK' : 'FAIL', $name, $summary);
exit($exit);
