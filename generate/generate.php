<?php
require_once '../config.php';

// Set content type to JSON
header('Content-Type: application/json');

// Only allow POST requests
if ($_SERVER['REQUEST_METHOD'] !== 'POST') {
    http_response_code(405);
    echo json_encode(['success' => false, 'message' => 'Method not allowed']);
    exit();
}

// Get form data
$redirectUrl = $_POST['redirectUrl'] ?? '';
$type = $_POST['type'] ?? 'public';
$email = $_POST['email'] ?? '';
$code = $_POST['code'] ?? '';
$expiresAt = $_POST['expiresAt'] ?? '';

// Convert expires_at to MySQL timestamp format if provided
if (!empty($expiresAt)) {
    // Convert from datetime-local format to MySQL timestamp format
    $expiresAtTimestamp = date('Y-m-d H:i:s', strtotime($expiresAt));
} else {
    $expiresAtTimestamp = null;
}

// Validate required fields
if (empty($redirectUrl) || empty($code)) {
    echo json_encode(['success' => false, 'message' => 'Missing required fields']);
    exit();
}

// Validate URL
if (!filter_var($redirectUrl, FILTER_VALIDATE_URL)) {
    echo json_encode(['success' => false, 'message' => 'Invalid URL']);
    exit();
}

// Validate email if provided
if (!empty($email) && !filter_var($email, FILTER_VALIDATE_EMAIL)) {
    echo json_encode(['success' => false, 'message' => 'Invalid email']);
    exit();
}

try {
    $pdo = getDbConnection();
    
    // Check if code already exists
    $stmt = $pdo->prepare("SELECT COUNT(*) FROM qr WHERE code = ?");
    $stmt->execute([$code]);
    if ($stmt->fetchColumn() > 0) {
        echo json_encode(['success' => false, 'message' => 'Code already exists, please try again']);
        exit();
    }
    
    // Insert new QR code record
    $stmt = $pdo->prepare("
        INSERT INTO qr (code, redirect_to, type, email, created_at, claimed_at, expires_at) 
        VALUES (?, ?, ?, ?, NOW(), NOW(), ?)
    ");
    
    $stmt->execute([$code, $redirectUrl, $type, $email, $expiresAtTimestamp]);
    
    // Return success response
    echo json_encode([
        'success' => true,
        'code' => $code,
        'redirect_to' => $redirectUrl,
        'type' => $type,
        'email' => $email,
        'expires_at' => $expiresAtTimestamp
    ]);
    
} catch (Exception $e) {
    echo json_encode(['success' => false, 'message' => 'Database error: ' . $e->getMessage()]);
}
?>