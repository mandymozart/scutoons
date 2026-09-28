<?php
require_once __DIR__ . '/config.php';

function generateUuid() {
    $data    = random_bytes(16);
    $data[6] = chr(ord($data[6]) & 0x0f | 0x40);
    $data[8] = chr(ord($data[8]) & 0x3f | 0x80);
    return vsprintf('%s%s-%s-%s-%s-%s%s%s', str_split(bin2hex($data), 4));
}

/**
 * Current time for the `qr` table columns (created_at, claimed_at, expires_at are written in the server's
 * timezone by generate.php). Use this instead of SQL NOW() so the same queries run on MySQL and SQLite.
 */
function qrNow() {
    return date('Y-m-d H:i:s');
}

/** True when the code has an expires_at in the past (same rule everywhere: landing page, validate, claim). */
function qrIsExpired(array $row) {
    if (empty($row['expires_at'])) return false;
    $expires = strtotime((string) $row['expires_at']);
    return $expires !== false && $expires < time();
}

// Default MySQL connection — config.local.php may define its own (e.g. SQLite for local dev)
if (!function_exists('getDbConnection')) {
    function getDbConnection() {
        static $pdo = null;
        if ($pdo) return $pdo;
        try {
            $pdo = new PDO(
                "mysql:host=" . DB_HOST . ";dbname=" . DB_NAME . ";charset=utf8mb4",
                DB_USER,
                DB_PASS,
                [
                    PDO::ATTR_ERRMODE            => PDO::ERRMODE_EXCEPTION,
                    PDO::ATTR_DEFAULT_FETCH_MODE => PDO::FETCH_ASSOC,
                    PDO::ATTR_EMULATE_PREPARES   => false,
                ]
            );
            return $pdo;
        } catch (PDOException $e) {
            error_log('[qr] database connection failed: ' . $e->getMessage());
            http_response_code(500);
            die('Service temporarily unavailable.');
        }
    }
}
