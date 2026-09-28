<?php
/**
 * Legacy membership validation (old viewer): POST { code } or GET ?code= / ?c=
 * → { status: "success", qr: { …row without email and signature…, is_claimed, is_expired } }
 *
 * SECURITY: never returns `email` (who claimed it) or `signature`. Expired codes answer 410.
 */
header('Access-Control-Allow-Origin: *');
header('Access-Control-Allow-Methods: GET, POST, OPTIONS');
header('Access-Control-Allow-Headers: Content-Type, Authorization');
header('Content-Type: application/json');
header('Cache-Control: no-store');

if ($_SERVER['REQUEST_METHOD'] === 'OPTIONS') {
    http_response_code(204);
    exit();
}

require_once __DIR__ . '/../../db.php';

function sendResponse($status, $data = null, $message = null) {
    $response = ['status' => $status];
    if ($status === 'success' && $data !== null) {
        $response['qr'] = $data;
    } elseif ($status === 'error' && $message !== null) {
        $response['message'] = $message;
    }
    echo json_encode($response);
    exit();
}

$code = $_GET['code'] ?? $_GET['c'] ?? null;
if ($_SERVER['REQUEST_METHOD'] === 'POST') {
    $input = json_decode((string) file_get_contents('php://input'), true);
    if (is_array($input)) {
        $code = $input['code'] ?? $input['c'] ?? $code;
    }
}

if (!is_string($code) || $code === '') {
    http_response_code(400);
    sendResponse('error', null, 'QR code parameter is required');
}
if (strlen($code) > 8) {
    http_response_code(400);
    sendResponse('error', null, 'Invalid QR code format');
}

try {
    $pdo = getDbConnection();
    $stmt = $pdo->prepare('SELECT * FROM qr WHERE code = ?');
    $stmt->execute([$code]);
    $result = $stmt->fetch();

    if (!$result) {
        http_response_code(404);
        sendResponse('error', null, 'QR code not found');
    }

    if (qrIsExpired($result)) {
        http_response_code(410);
        sendResponse('error', null, 'QR code expired');
    }

    $isClaimed = !empty($result['email']) || !empty($result['claimed_by']);

    // Never expose who claimed the code or the signature blob
    unset($result['email'], $result['signature'], $result['claimed_by']);

    if (isset($result['created_at'])) {
        $result['created_at'] = date('c', strtotime($result['created_at']));
    }
    if (!empty($result['claimed_at'])) {
        $result['claimed_at'] = date('c', strtotime($result['claimed_at']));
    }
    $result['is_claimed'] = $isClaimed;
    $result['is_expired'] = false;

    http_response_code(200);
    sendResponse('success', $result);
} catch (Throwable $e) {
    error_log('[qr] validate: ' . $e->getMessage());
    http_response_code(500);
    sendResponse('error', null, 'An error occurred while validating the QR code');
}
