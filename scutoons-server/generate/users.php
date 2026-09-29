<?php
/**
 * Admin: reader accounts and their roles (ITEMS.md "Roles, signup rule, promo codes"). Admin token required.
 *   GET users.php?q=<part of an email>[&limit=50]
 *     → { success, roles: [{ id, name, description }], users: [{ id, email, created_at, roles, signup_code,
 *         marketing_opt_in, marketing_opt_in_at }], total }
 * Empty q = the newest accounts. Roles are changed with user-roles.php.
 */
require_once __DIR__ . '/auth-check.php';
require_once __DIR__ . '/../api/lib/bootstrap.php';

header('Content-Type: application/json');
header('Cache-Control: no-store');

$q = is_string($_GET['q'] ?? null) ? strtolower(trim(mb_substr($_GET['q'], 0, 254))) : '';
$limit = max(1, min(200, (int) ($_GET['limit'] ?? 50)));

try {
    $where = '';
    $params = [];
    if ($q !== '') {
        $where = "WHERE email LIKE ? ESCAPE '!'";
        $params[] = '%' . strtr($q, ['!' => '!!', '%' => '!%', '_' => '!_']) . '%';
    }
    $total = (int) Db::value("SELECT COUNT(*) FROM users $where", $params);
    $rows = Db::all("SELECT id, email, created_at, signup_code, marketing_opt_in, marketing_opt_in_at FROM users $where ORDER BY created_at DESC, email LIMIT $limit", $params);
    echo json_encode([
        'success' => true,
        'roles' => Roles::all(),
        'total' => $total,
        'users' => array_map([Users::class, 'adminJson'], $rows),
    ], JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
} catch (Throwable $e) {
    error_log('[qr] users: ' . $e->getMessage());
    http_response_code(500);
    echo json_encode(['success' => false, 'message' => 'Could not load the users']);
}
