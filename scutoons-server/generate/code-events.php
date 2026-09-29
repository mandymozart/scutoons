<?php
/**
 * Admin: one code with its event timeline. Admin token required.
 *   GET code-events.php?code=CODE → { success, code: {...}, events: [{ event, created_at, device, account, toon }] }
 * device = first 10 characters of the device hash, account = whether a signed-in account was involved
 * (never the email or user id).
 */
require_once __DIR__ . '/auth-check.php';
require_once __DIR__ . '/../api/lib/bootstrap.php';

header('Content-Type: application/json');
header('Cache-Control: no-store');

$code = Codes::normalize($_GET['code'] ?? null);
if ($code === null) {
    http_response_code(400);
    echo json_encode(['success' => false, 'message' => 'Missing code']);
    exit;
}

try {
    $qr = Db::one('SELECT code, type, edition_id, edition_number, redirect_to, created_at, claimed_at, claimed_by, email, expires_at, revoked_at, revoked_reason FROM qr WHERE code = ?', [$code]);
    if (!$qr) {
        http_response_code(404);
        echo json_encode(['success' => false, 'message' => 'Code not found']);
        exit;
    }
    $claimed = $qr['claimed_by'] !== null || (string) $qr['email'] !== '';
    unset($qr['claimed_by'], $qr['email']);
    $qr['claimed'] = $claimed;
    if (!$claimed) $qr['claimed_at'] = null;

    $events = array_map(fn($e) => [
        'event' => $e['event'],
        'created_at' => Db::iso($e['created_at']),
        'device' => $e['device_hash'] !== null ? substr($e['device_hash'], 0, 10) : null,
        'account' => $e['user_id'] !== null,
        'toon' => $e['toon'],
    ], Db::all('SELECT event, created_at, device_hash, user_id, toon FROM code_events WHERE code = ? ORDER BY created_at DESC, id DESC LIMIT 1000', [$code]));

    echo json_encode(['success' => true, 'code' => $qr, 'events' => $events]);
} catch (Throwable $e) {
    error_log('[qr] code-events: ' . $e->getMessage());
    http_response_code(500);
    echo json_encode(['success' => false, 'message' => 'Could not load events']);
}
