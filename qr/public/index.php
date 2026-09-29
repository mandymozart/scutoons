<?php
declare(strict_types=1);

/**
 * Front controller of qr.scutoons.com: every request that is not a static file in public/ (verify.html, assets/,
 * version.json) – the QR landing page and the proxies to the API (src/App.php).
 */
ini_set('display_errors', '0');
require __DIR__ . '/../src/bootstrap.php';

Qr\App::fromConfig()->run();
