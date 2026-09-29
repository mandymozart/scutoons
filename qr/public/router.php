<?php
/**
 * Router for the PHP built-in server (local development only – Apache uses public/.htaccess):
 *
 *   php -S localhost:9001 -t public public/router.php      (composer serve)
 *
 * Static files in public/ are served as they are (verify.html, assets/); everything else goes to the front
 * controller. Dot files and ".." are never served.
 */
if (PHP_SAPI !== 'cli-server') {
    http_response_code(404);
    exit;
}

$path = rawurldecode((string) parse_url($_SERVER['REQUEST_URI'] ?? '/', PHP_URL_PATH));
if (str_contains($path, '..') || preg_match('#(^|/)\.#', $path)) {
    http_response_code(404);
    echo 'Not found';
    return true;
}
$file = __DIR__ . $path;
if ($path !== '/' && is_file($file) && !str_ends_with($file, '.php')) return false;

$_SERVER['SCRIPT_NAME'] = '/index.php';
$_SERVER['SCRIPT_FILENAME'] = __DIR__ . '/index.php';
require __DIR__ . '/index.php';
return true;
