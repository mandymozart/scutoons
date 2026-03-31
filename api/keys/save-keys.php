<?php
/**
 * Save Generated Keys
 * Called by generate-keys.php after keys are generated
 */

header('Content-Type: application/json');

// Only accept POST requests
if ($_SERVER['REQUEST_METHOD'] !== 'POST') {
    echo json_encode(['success' => false, 'error' => 'Method not allowed']);
    exit;
}

$keysFile = __DIR__ . '/keypair.json';

// Check if keys already exist (prevent overwrite)
if (file_exists($keysFile)) {
    echo json_encode([
        'success' => false, 
        'error' => 'Keys already exist. Delete keypair.json first if you want to regenerate.'
    ]);
    exit;
}

// Get posted data
$input = file_get_contents('php://input');
$data = json_decode($input, true);

// Validate input
if (!$data || !isset($data['privateKey']) || !isset($data['publicKey'])) {
    echo json_encode(['success' => false, 'error' => 'Invalid data format']);
    exit;
}

// Validate key lengths (ML-DSA-65 specific)
$privateKeyLength = strlen($data['privateKey']) / 2; // hex to bytes
$publicKeyLength = strlen($data['publicKey']) / 2;

if ($privateKeyLength !== 4032) {
    echo json_encode([
        'success' => false, 
        'error' => "Invalid private key length: $privateKeyLength bytes (expected 4032)"
    ]);
    exit;
}

if ($publicKeyLength !== 1952) {
    echo json_encode([
        'success' => false, 
        'error' => "Invalid public key length: $publicKeyLength bytes (expected 1952)"
    ]);
    exit;
}

// Prepare key data
$keyData = [
    'privateKey' => $data['privateKey'],
    'publicKey' => $data['publicKey'],
    'algorithm' => 'ML-DSA-65',
    'generated' => $data['generated'] ?? date('c'),
    'note' => 'This file contains the post-quantum cryptographic keypair for SCUTOONS. Keep it secure!'
];

// Save to file
$saved = file_put_contents($keysFile, json_encode($keyData, JSON_PRETTY_PRINT));

if ($saved === false) {
    echo json_encode(['success' => false, 'error' => 'Failed to write keys to file']);
    exit;
}

// Set restrictive permissions
chmod($keysFile, 0600); // Owner read/write only

echo json_encode([
    'success' => true,
    'file' => $keysFile,
    'message' => 'Keys saved successfully'
]);
exit;
?>
