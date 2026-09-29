<?php
declare(strict_types=1);

/**
 * Shared bootstrap for the accounts/items API (api/index.php, api/items/) and the landing page:
 * config + DB (db.php), the classes in api/lib/, and the API error log.
 */
require_once __DIR__ . '/../../db.php';

foreach (['Config', 'Http', 'Db', 'Catalog', 'Permissions', 'Events', 'RateLimit', 'Mailer', 'Smtp', 'LoginMail', 'Users', 'Roles', 'Auth', 'Codes', 'Inventory', 'Avatar', 'Progress'] as $class) {
    require_once __DIR__ . "/$class.php";
}

$logPath = Config::get('LOG_PATH');
if ($logPath !== '') {
    $dir = dirname($logPath);
    if (is_dir($dir) || @mkdir($dir, 0770, true)) {
        ini_set('log_errors', '1');
        ini_set('error_log', $logPath);
    }
}
ini_set('display_errors', '0');
