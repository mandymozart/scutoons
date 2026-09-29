<?php
/**
 * Router for the PHP built-in server (local development only – Apache uses public/.htaccess):
 *
 *   php -S localhost:9000 -t public public/router.php      (composer serve)
 *
 * Static files in public/ are served as they are (verify.html, assets/, the generate/ SPA files), directories with an
 * index.html get it; everything else goes to the front controller. Dot files and ".." are never served.
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
if ($path !== '/' && is_dir($file) && is_file(rtrim($file, '/') . '/index.html')) {
    if (!str_ends_with($path, '/')) {
        header('Location: ' . $path . '/' . (isset($_SERVER['QUERY_STRING']) && $_SERVER['QUERY_STRING'] !== '' ? '?' . $_SERVER['QUERY_STRING'] : ''), true, 301);
        return true;
    }
    return false;
}

$_SERVER['SCRIPT_NAME'] = '/index.php';
$_SERVER['SCRIPT_FILENAME'] = __DIR__ . '/index.php';
require __DIR__ . '/index.php';
return true;
