<?php
/**
 * Retrieve QR Code Signature from Database
 * Returns the signature data for verification
 */
require_once '../config.php';

// Set content type to JSON
header('Content-Type: application/json');

// Only allow GET requests
if ($_SERVER['REQUEST_METHOD'] !== 'GET') {
    http_response_code(405);
    echo json_encode(['success' => false, 'message' => 'Method not allowed']);
    exit();
}

// Get code from query parameter
$code = $_GET['code'] ?? '';

if (empty($code)) {
    echo json_encode(['success' => false, 'message' => 'Missing code parameter']);
    exit();
}

try {
    $pdo = getDbConnection();
    
    // Retrieve signature only - everything is stored in the signature JSON
    $stmt = $pdo->prepare("
        SELECT signature, redirect_to
        FROM qr 
        WHERE code = ?
    ");
    $stmt->execute([$code]);
    $qr = $stmt->fetch(PDO::FETCH_ASSOC);
    
    if (!$qr) {
        http_response_code(404);
        echo json_encode(['success' => false, 'message' => 'QR code not found']);
        exit();
    }
    
    if (empty($qr['signature'])) {
        http_response_code(404);
        echo json_encode(['success' => false, 'message' => 'No signature found for this QR code']);
        exit();
    }
    
    // Parse signature JSON (contains complete signature data)
    $signatureData = json_decode($qr['signature'], true);
    
    if (!$signatureData) {
        echo json_encode(['success' => false, 'message' => 'Invalid signature data']);
        exit();
    }
    
    // Generate fingerprint from signature (first 32 bytes in hex format)
    $fingerprint = '';
    if (isset($signatureData['signature'])) {
        $fingerprintBytes = substr($signatureData['signature'], 0, 64); // First 32 bytes (64 hex chars)
        $fingerprint = strtoupper(implode(':', str_split($fingerprintBytes, 4)));
    }
    
    // Parse signedData to get full QR data (includes template fields)
    $signedDataParsed = json_decode($signatureData['signedData'], true);
    
    // Extract template fields from signedData (no duplication)
    // Template fields are: receiver, productname, edition_number, edition_count, creators, etc.
    // They're already in signedData, so we just pass signedDataParsed to frontend
    
    // Return complete data for verification AND regeneration
    echo json_encode([
        'success' => true,
        'signature' => $signatureData,  // Contains signedData with ALL fields
        'fingerprint' => $fingerprint,
        'qr_data' => $signedDataParsed,  // ALL data including template fields
        'redirect_to' => $qr['redirect_to']
    ]);
    
} catch (Exception $e) {
    http_response_code(500);
    echo json_encode([
        'success' => false, 
        'message' => 'Database error: ' . $e->getMessage()
    ]);
}
?>
