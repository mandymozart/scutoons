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

// SECURITY: Only allow key access from generate/ folder (htaccess protected)
// Check referer to ensure request comes from protected area
$referer = $_SERVER['HTTP_REFERER'] ?? '';
$host = $_SERVER['HTTP_HOST'] ?? '';

if (!str_contains($referer, $host . '/generate/')) {
    http_response_code(403);
    echo json_encode([
        'success' => false,
        'error' => 'Access denied. Keys only accessible from protected generate area.'
    ]);
    exit;
}

// Return keys (protected by htaccess + referer check)
// Note: Client-side signing is necessary because ML-DSA-65 has no mature PHP implementation
echo json_encode([
    'success' => true,
    'privateKey' => $keys['privateKey'],
    'publicKey' => $keys['publicKey'],
    'generated' => $keys['generated'] ?? null,
    'note' => 'Protected: generate/ folder requires authentication via htaccess'
]);
exit;
?>
