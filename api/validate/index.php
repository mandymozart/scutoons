<?php
declare(strict_types=1);

/**
 * Membership validation: POST { code } or GET ?code= / ?c=, with `Authorization: Bearer <session>`.
 *
 * A code belongs to exactly one account. This endpoint only confirms a code for the account that claimed it,
 * so a photographed or shared code can't unlock anything on another device:
 *   no session                → 401 login_required
 *   code unclaimed            → 403 not_claimed      (claim it via /inventory/claim/ first)
 *   claimed by someone else   → 409 already_claimed
 *   claimed by the caller     → 200 { status: "success", qr: { …row without email/signature/claimed_by… } }
 * Unknown → 404, expired → 410. Never returns `email`, `signature` or `claimed_by`.
 */
require_once __DIR__ . '/../lib/bootstrap.php';

Http::run(function () {
    $method = $_SERVER['REQUEST_METHOD'] ?? 'GET';
    if ($method !== 'GET' && $method !== 'POST') throw new ApiError(405, 'method_not_allowed', 'Use GET or POST.');

    $input = $method === 'POST' ? Http::body() : [];
    $code = Codes::normalize($input['code'] ?? $input['c'] ?? $_GET['code'] ?? $_GET['c'] ?? null)
        ?? throw new ApiError(400, 'invalid_code', 'QR code parameter is missing or malformed.');

    $row = Codes::find($code) ?? throw new ApiError(404, 'not_found', 'QR code not found.');
    if (qrIsExpired($row)) throw new ApiError(410, 'expired', 'QR code expired.');

    $user = Auth::optionalUser()
        ?? throw new ApiError(401, 'login_required', 'Sign in to use this code.');
    if ($row['claimed_by'] === null) {
        throw new ApiError(403, 'not_claimed', 'Add this code to your inventory first.');
    }
    if ($row['claimed_by'] !== $user['id']) {
        throw new ApiError(409, 'already_claimed', 'This code belongs to another account.');
    }

    unset($row['email'], $row['signature'], $row['claimed_by']);
    foreach (['created_at', 'claimed_at'] as $field) {
        if (!empty($row[$field])) $row[$field] = date('c', (int) strtotime((string) $row[$field]));
    }
    $row['is_claimed'] = true;
    $row['is_expired'] = false;
    Http::json(200, ['status' => 'success', 'qr' => $row]);
});
