<?php
/**
 * Key Management for Post-Quantum Signatures
 * This file stores and retrieves the ML-DSA-65 keypair
 * 
 * SECURITY WARNING: This is a simplified implementation.
 * For production, use:
 * - Environment variables
 * - Hardware Security Module (HSM)
 * - Encrypted key storage
 * - Access control (authentication required)
 */

header('Content-Type: application/json');

$keysFile = __DIR__ . '/keypair.json';

// Check if keys exist
if (!file_exists($keysFile)) {
    // Keys don't exist yet - need to generate them
    echo json_encode([
        'success' => false,
        'error' => 'Keys not initialized. Please run generate-keys.php first.',
        'needsGeneration' => true
    ]);
    exit;
}

// Load existing keys
$keys = json_decode(file_get_contents($keysFile), true);

if (!$keys || !isset($keys['privateKey']) || !isset($keys['publicKey'])) {
    echo json_encode([
        'success' => false,
        'error' => 'Invalid key file format'
    ]);
    exit;
}

// Return keys
// NOTE: In production, NEVER send private key to client!
// This is a temporary solution. Move signing to server-side.
echo json_encode([
    'success' => true,
    'privateKey' => $keys['privateKey'],
    'publicKey' => $keys['publicKey'],
    'generated' => $keys['generated'] ?? null,
    'warning' => 'DEVELOPMENT MODE: Private key exposed to client. Use server-side signing in production!'
]);
exit;
?>
