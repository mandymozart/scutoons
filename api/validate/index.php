<?php
declare(strict_types=1);

/**
 * Legacy membership validation for the old viewer: POST { code } or GET ?code= / ?c= (no login).
 * Backwards compatible (ITEMS.md "Access model", final): success for any existing, non-expired code – claimed or
 * not – because viewing is device-level; owning a code is a separate account action (/inventory/claim/).
 *   → 200 { status: "success", qr: { …row without email / signature / claimed_by…, is_claimed, is_expired } }
 *   unknown → 404, expired → 410, both { status: "error", message }.
 * Never returns `email`, `signature` or `claimed_by`. CORS `*` like before.
 */
require_once __DIR__ . '/../lib/bootstrap.php';

header('Access-Control-Allow-Origin: *');
header('Access-Control-Allow-Methods: GET, POST, OPTIONS');
header('Access-Control-Allow-Headers: Content-Type, Authorization');

Http::run(function () {
    $method = $_SERVER['REQUEST_METHOD'] ?? 'GET';
    if ($method !== 'GET' && $method !== 'POST') throw new ApiError(405, 'method_not_allowed', 'Use GET or POST.');

    $input = $method === 'POST' ? Http::body() : [];
    $code = Codes::normalize($input['code'] ?? $input['c'] ?? $_GET['code'] ?? $_GET['c'] ?? null)
        ?? throw new ApiError(400, 'invalid_code', 'QR code parameter is required');

    $row = Codes::find($code) ?? throw new ApiError(404, 'not_found', 'QR code not found');
    if (qrIsExpired($row)) throw new ApiError(410, 'expired', 'QR code expired');

    $isClaimed = Codes::isClaimed($row);
    unset($row['email'], $row['signature'], $row['claimed_by']);
    foreach (['created_at', 'claimed_at'] as $field) {
        if (!empty($row[$field])) $row[$field] = date('c', (int) strtotime((string) $row[$field]));
    }
    $row['is_claimed'] = $isClaimed;
    $row['is_expired'] = false;
    Http::json(200, ['status' => 'success', 'qr' => $row]);
});
