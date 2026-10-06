<?php
declare(strict_types=1);

/**
 * Front controller of qr.scutoons.com: every request that is not a static file in public/ (verify.html, assets/,
 * version.json) – the QR landing page and the proxies to the API (src/App.php).
 */
ini_set('display_errors', '0');
// PHP errors go to qr/logs/error.log (never served – .htaccess; the admin's "Server errors" page reads it, BACKLOG 10.11)
if (is_dir(__DIR__ . '/../logs') || @mkdir(__DIR__ . '/../logs', 0770, true)) {
    ini_set('log_errors', '1');
    ini_set('error_log', __DIR__ . '/../logs/error.log');
}
require __DIR__ . '/../src/bootstrap.php';

Qr\App::fromConfig()->run();
