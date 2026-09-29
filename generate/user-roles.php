<?php
/**
 * Admin: grant or revoke a role. Admin token required.
 *   POST user-roles.php { user_id | email, role, action: "grant" | "revoke" } → { success, changed, user }
 * `member` can be revoked too (every new account gets it again at signup only). Unknown role → 400.
 */
require_once __DIR__ . '/auth-check.php';
require_once __DIR__ . '/../api/lib/bootstrap.php';

header('Content-Type: application/json');
header('Cache-Control: no-store');

function fail(int $status, string $message): never
{
    http_response_code($status);
    echo json_encode(['success' => false, 'message' => $message]);
    exit;
}

if (($_SERVER['REQUEST_METHOD'] ?? 'GET') !== 'POST') fail(405, 'Method not allowed');

$body = json_decode((string) file_get_contents('php://input'), true) ?: [];
$role = is_string($body['role'] ?? null) ? $body['role'] : '';
$action = $body['action'] ?? '';
if (!in_array($action, ['grant', 'revoke'], true) || $role === '') fail(400, 'role and action (grant|revoke) required');

try {
    if (!Roles::exists($role)) fail(400, "Unknown role $role");
    $user = is_string($body['user_id'] ?? null) ? Users::byId($body['user_id'])
        : (is_string($body['email'] ?? null) ? Users::byEmail(strtolower(trim($body['email']))) : null);
    if (!$user) fail(404, 'User not found');
    $changed = $action === 'grant' ? Roles::grant($user['id'], $role, 'admin') : Roles::revoke($user['id'], $role);
    echo json_encode(['success' => true, 'changed' => $changed, 'user' => Users::adminJson(Users::byId($user['id']))], JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
} catch (Throwable $e) {
    error_log('[qr] user-roles: ' . $e->getMessage());
    fail(500, 'Could not change the role');
}
