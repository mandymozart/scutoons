<?php
/**
 * Full ML-DSA-65 keypair for the admin generate SPA (client-side signing, see api/keys/README.md).
 * Requires the admin token (X-Generate-Token, auth-check.php) on top of the generate/ Basic auth.
 * The public /api/keys/keys.php only returns the public key.
 */
require_once __DIR__ . '/auth-check.php';

header('Content-Type: application/json');
header('Cache-Control: no-store');

$keysFile = __DIR__ . '/../api/keys/keypair.json';

if (!is_file($keysFile)) {
    echo json_encode([
        'success' => false,
        'error' => 'Keys not initialized. See api/keys/README.md.',
        'needsGeneration' => true,
    ]);
    exit;
}

$keys = json_decode((string) file_get_contents($keysFile), true);
if (!is_array($keys) || !isset($keys['privateKey'], $keys['publicKey'])) {
    error_log('[qr] generate/keys.php: invalid keypair.json');
    http_response_code(500);
    echo json_encode(['success' => false, 'error' => 'Invalid key file']);
    exit;
}

echo json_encode([
    'success'    => true,
    'privateKey' => $keys['privateKey'],
    'publicKey'  => $keys['publicKey'],
    'generated'  => $keys['generated'] ?? null,
]);
