<?php
/**
 * Runs every HTTP contract suite against freshly started dev servers on a COPY of the local SQLite database
 * (composer test:contract; part of composer test). Nothing touches the configured database.
 *
 *   php tests/run-contract.php [--port=9180] [--keep]
 *
 * 1. copies DB_SQLITE_PATH (config.local.php) to a temp file
 * 2. starts `php -S 127.0.0.1:<port> -t public public/router.php` (QR_DB_SQLITE_PATH = the copy) and a second one on
 *    <port+1> with the transition flag off (QR_QR_HOST_SERVES_API=0) for the host-routing checks
 * 3. runs api-smoke, content-smoke, claim-race (claim + signup), admin-smoke, public-urls (+ --hosts) and the
 *    host checks with the flag off
 * 4. stops the servers and deletes the copy (--keep keeps it and prints its path)
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

$env = getenv() + [];
$env['QR_DB_SQLITE_PATH'] = $copy;
$servers = [];
$start = function (int $port, array $extraEnv) use ($root, $env, &$servers): void {
    $log = str_replace('\\', '/', sys_get_temp_dir()) . "/scutoons-contract-server-$port.log";
    $proc = proc_open([PHP_BINARY, '-S', "127.0.0.1:$port", '-t', "$root/public", "$root/public/router.php"],
        [0 => ['file', 'php://stdin', 'r'], 1 => ['file', $log, 'w'], 2 => ['file', $log, 'a']], $pipes, $root, $extraEnv + $env);
    if (!is_resource($proc)) throw new RuntimeException("could not start the server on :$port");
    $servers[] = $proc;
    for ($i = 0; $i < 100; $i++) {
        $ok = @file_get_contents("http://127.0.0.1:$port/api/health/");
        if ($ok !== false) return;
        usleep(100000);
    }
    throw new RuntimeException("server on :$port did not answer (log: $log)");
};

$results = [];
$exit = 0;
try {
    $start($port, []);
    $start($port + 1, ['QR_QR_HOST_SERVES_API' => '0']);
    $base = "http://127.0.0.1:$port";
    $suites = [
        'api-smoke' => ['tests/api-smoke.php', $base],
        'content-smoke' => ['tests/content-smoke.php', $base],
        'claim-race (claim)' => ['tests/claim-race.php', '8', 'claim'],
        'claim-race (signup)' => ['tests/claim-race.php', '8', 'signup'],
        'admin-smoke' => ['tests/admin-smoke.php', $base],
        'public-urls (+ hosts)' => ['tests/public-urls.php', $base, '--hosts'],
        'public-urls (hosts, QR_HOST_SERVES_API=0)' => ['tests/public-urls.php', 'http://127.0.0.1:' . ($port + 1), '--hosts=only', '--qr-api=off'],
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
    if ($keep) echo "\nKept the database copy: $copy\n";
    else @unlink($copy);
}

echo "\nContract suites\n";
foreach ($results as $name => [$code, $summary]) echo sprintf("  %-4s %-44s %s\n", $code === 0 ? 'OK' : 'FAIL', $name, $summary);
exit($exit);
