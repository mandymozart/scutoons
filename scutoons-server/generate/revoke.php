<?php
/**
 * Admin: revoke or restore one code. Admin token required.
 *   POST revoke.php { code, action: "revoke" | "unrevoke", reason? } → { success, code, revoked_at, revoked_reason }
 * A revoked code grants nothing (/access/, /validate/, claim → 410 revoked; /me ignores it). Logged as an event.
 */
require_once __DIR__ . '/auth-check.php';
require_once __DIR__ . '/../api/lib/bootstrap.php';

header('Content-Type: application/json');
header('Cache-Control: no-store');

if (($_SERVER['REQUEST_METHOD'] ?? 'GET') !== 'POST') {
    http_response_code(405);
    echo json_encode(['success' => false, 'message' => 'Method not allowed']);
    exit;
}

$body = json_decode((string) file_get_contents('php://input'), true) ?: [];
$code = Codes::normalize($body['code'] ?? null);
$action = $body['action'] ?? '';
$reason = is_string($body['reason'] ?? null) ? trim(mb_substr($body['reason'], 0, 255)) : '';
if ($code === null || !in_array($action, ['revoke', 'unrevoke'], true)) {
    http_response_code(400);
    echo json_encode(['success' => false, 'message' => 'code and action (revoke|unrevoke) required']);
    exit;
}

try {
    if (!Codes::find($code)) {
        http_response_code(404);
        echo json_encode(['success' => false, 'message' => 'Code not found']);
        exit;
    }
    if ($action === 'revoke') {
        Db::run('UPDATE qr SET revoked_at = ?, revoked_reason = ? WHERE code = ?', [qrNow(), $reason !== '' ? $reason : null, $code]);
    } else {
        Db::run('UPDATE qr SET revoked_at = NULL, revoked_reason = NULL WHERE code = ?', [$code]);
    }
    Events::log($action, $code);
    $row = Codes::find($code);
    echo json_encode(['success' => true, 'code' => $code, 'revoked_at' => $row['revoked_at'], 'revoked_reason' => $row['revoked_reason']]);
} catch (Throwable $e) {
    error_log('[qr] revoke: ' . $e->getMessage());
    http_response_code(500);
    echo json_encode(['success' => false, 'message' => 'Could not update the code']);
}
