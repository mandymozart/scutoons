<?php
/**
 * Public key of the ML-DSA-65 keypair (for verifying certificate signatures).
 *
 * SECURITY: this endpoint never returns the private key. The admin generate SPA loads the full keypair from
 * /generate/keys.php, which requires the admin token (X-Generate-Token) and sits behind the generate/ Basic auth.
 */

header('Content-Type: application/json');
header('Cache-Control: no-store');

$keysFile = __DIR__ . '/keypair.json';
$keys = is_file($keysFile) ? json_decode((string) file_get_contents($keysFile), true) : null;

if (!is_array($keys) || !isset($keys['publicKey'])) {
    http_response_code(404);
    echo json_encode(['success' => false, 'error' => 'Public key not available']);
    exit;
}

echo json_encode([
    'success'   => true,
    'publicKey' => $keys['publicKey'],
    'algorithm' => $keys['algorithm'] ?? 'ML-DSA-65',
    'generated' => $keys['generated'] ?? null,
]);
