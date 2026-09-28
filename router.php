<?php
/**
 * Router for the PHP built-in server (local development only – Apache uses .htaccess):
 *
 *   php -S localhost:9000 router.php
 *
 * Mirrors .htaccess: denies secrets/local data, sends unknown /api/ paths to api/index.php, existing files and
 * folders are served as-is, everything else goes to index.php (landing page).
 */
if (PHP_SAPI !== 'cli-server') {
    http_response_code(404);
    exit;
}

$path = rawurldecode((string) parse_url($_SERVER['REQUEST_URI'] ?? '/', PHP_URL_PATH));
$file = __DIR__ . $path;

// Never serve secrets, local databases, logs or CLI-only files
if (preg_match('#(^|/)(\.|db/|logs/|scripts/|tests/|api/lib/)|keypair\.json$|\.htpasswd$|\.sqlite|\.log$|^/(config(\.local)?\.php|config\.local\.php\.example|db\.php|router\.php)$#', $path)
    || str_contains($path, '..')) {
    http_response_code(404);
    echo 'Not found';
    return true;
}
// api/keys: only keys.php (public key), like api/keys/.htaccess
if (str_starts_with($path, '/api/keys/') && $path !== '/api/keys/keys.php') {
    http_response_code(404);
    echo 'Not found';
    return true;
}

if (str_starts_with($path, '/api/') || $path === '/api') {
    if (is_file($file)) return false;
    if (is_dir($file) && is_file(rtrim($file, '/') . '/index.php')) {
        // Folder endpoints (/api/validate/, /api/items/ …): run their index.php
        $_SERVER['SCRIPT_NAME'] = rtrim($path, '/') . '/index.php';
        chdir(rtrim($file, '/'));
        require rtrim($file, '/') . '/index.php';
        return true;
    }
    chdir(__DIR__ . '/api');
    require __DIR__ . '/api/index.php';
    return true;
}

if ($path !== '/' && (is_file($file) || is_dir($file))) {
    return false;
}

chdir(__DIR__);
require __DIR__ . '/index.php';
return true;
