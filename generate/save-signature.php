<?php
/**
 * Save QR Code Signature to Database
 * Updates the signature field for an existing QR code
 */
require_once '../config.php';

// Set content type to JSON
header('Content-Type: application/json');

// Only allow POST requests
if ($_SERVER['REQUEST_METHOD'] !== 'POST') {
    http_response_code(405);
    echo json_encode(['success' => false, 'message' => 'Method not allowed']);
    exit();
}

// Get JSON input
$input = file_get_contents('php://input');
$data = json_decode($input, true);

// Validate input
if (!$data || !isset($data['code']) || !isset($data['signature'])) {
    echo json_encode(['success' => false, 'message' => 'Missing required fields: code and signature']);
    exit();
}

$code = $data['code'];
$signatureData = $data['signature'];

// Validate signature data structure
if (!isset($signatureData['signature']) || !isset($signatureData['publicKey']) || !isset($signatureData['algorithm'])) {
    echo json_encode(['success' => false, 'message' => 'Invalid signature data structure']);
    exit();
}

try {
    $pdo = getDbConnection();
    
    // Check if QR code exists
    $stmt = $pdo->prepare("SELECT id FROM qr WHERE code = ?");
    $stmt->execute([$code]);
    $qr = $stmt->fetch(PDO::FETCH_ASSOC);
    
    if (!$qr) {
        echo json_encode(['success' => false, 'message' => 'QR code not found']);
        exit();
    }
    
    // Store signature data as-is (already complete from frontend)
    // Contains: signature, publicKey, algorithm, signedData, timestamp
    $signatureJson = json_encode($signatureData);
    
    // Update the signature field
    $stmt = $pdo->prepare("
        UPDATE qr 
        SET signature = ? 
        WHERE code = ?
    ");
    
    $stmt->execute([$signatureJson, $code]);
    
    // Return success response
    echo json_encode([
        'success' => true,
        'message' => 'Signature saved to database',
        'code' => $code
    ]);
    
} catch (Exception $e) {
    echo json_encode([
        'success' => false, 
        'message' => 'Database error: ' . $e->getMessage()
    ]);
}
?>
