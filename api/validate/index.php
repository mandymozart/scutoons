<?php
// Enable CORS for cross-domain requests
header('Access-Control-Allow-Origin: *');
header('Access-Control-Allow-Methods: GET, POST, OPTIONS');
header('Access-Control-Allow-Headers: Content-Type, Authorization');
header('Content-Type: application/json');

// Handle preflight requests
if ($_SERVER['REQUEST_METHOD'] === 'OPTIONS') {
    http_response_code(200);
    exit();
}

// Include database configuration
require_once '../../config.php';

// Function to send JSON response
function sendResponse($status, $data = null, $message = null) {
    $response = ['status' => $status];
    
    if ($status === 'success' && $data !== null) {
        $response['qr'] = $data;
    } elseif ($status === 'error' && $message !== null) {
        $response['message'] = $message;
    }
    
    echo json_encode($response);
    exit();
}

// Check if code parameter is provided
$code = null;

// Check GET parameters
if (isset($_GET['code'])) {
    $code = $_GET['code'];
} elseif (isset($_GET['c'])) {
    $code = $_GET['c'];
}

// Check POST parameters (for JSON requests)
if ($_SERVER['REQUEST_METHOD'] === 'POST') {
    $input = json_decode(file_get_contents('php://input'), true);
    if (isset($input['code'])) {
        $code = $input['code'];
    } elseif (isset($input['c'])) {
        $code = $input['c'];
    }
}

// Validate that code is provided
if (empty($code)) {
    http_response_code(400);
    sendResponse('error', null, 'QR code parameter is required');
}

// Validate code format (should be 8 characters based on the schema)
if (strlen($code) > 8) {
    http_response_code(400);
    sendResponse('error', null, 'Invalid QR code format');
}

try {
    // Get database connection
    $pdo = getDbConnection();
    
    // Query database for matching code
    $stmt = $pdo->prepare("SELECT * FROM qr WHERE code = ?");
    $stmt->execute([$code]);
    $result = $stmt->fetch();
    
    if ($result) {
        // QR code found - return success with full QR data
        http_response_code(200);
        
        // Convert timestamps to ISO format for better JSON compatibility
        if (isset($result['created_at'])) {
            $result['created_at'] = date('c', strtotime($result['created_at']));
        }
        if (isset($result['claimed_at']) && $result['claimed_at']) {
            $result['claimed_at'] = date('c', strtotime($result['claimed_at']));
        }
        
        // Add validation status fields for convenience
        $result['is_claimed'] = !empty($result['email']);
        
        sendResponse('success', $result);
    } else {
        // QR code not found
        http_response_code(404);
        sendResponse('error', null, 'QR code not found');
    }
    
} catch (PDOException $e) {
    // Database error
    http_response_code(500);
    sendResponse('error', null, 'Database error occurred');
} catch (Exception $e) {
    // General error
    http_response_code(500);
    sendResponse('error', null, 'An error occurred while validating the QR code');
}
?>