<?php
// `php -S` on a local dev port from shared/dev-ports.json (the single source of truth), for composer / npm scripts
// that cannot read the file themselves:
//   php scripts/dev/php-serve.php <service> <docroot> <router>     e.g.  qr qr/public qr/public/router.php
// <docroot> and <router> are relative to the current folder (as with php -S).
declare(strict_types=1);

[, $service, $docroot, $router] = $argv + [null, null, null, null];
if ($service === null || $docroot === null || $router === null) {
    fwrite(STDERR, "usage: php scripts/dev/php-serve.php <service> <docroot> <router>\n");
    exit(2);
}
$ports = json_decode((string) file_get_contents(dirname(__DIR__, 2) . '/shared/dev-ports.json'), true);
$port = is_array($ports) ? ($ports[$service] ?? null) : null;
if (!is_int($port)) {
    fwrite(STDERR, "unknown dev service \"$service\" (shared/dev-ports.json)\n");
    exit(2);
}
passthru(implode(' ', array_map('escapeshellarg', [PHP_BINARY, '-S', "localhost:$port", '-t', $docroot, $router])), $code);
exit($code);
