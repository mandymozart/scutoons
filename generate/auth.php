<?php
require_once '../config.php';
header('Content-Type: application/json');

$body   = json_decode(file_get_contents('php://input'), true) ?? [];
$action = $body['action'] ?? '';

function generateToken() {
    return hash_hmac('sha256', 'generate-access', GENERATE_PASSWORD);
}

if ($action === 'login') {
    $password = $body['password'] ?? '';
    if (hash_equals(GENERATE_PASSWORD, $password)) {
        echo json_encode(['token' => generateToken()]);
    } else {
        http_response_code(401);
        echo json_encode(['error' => 'Invalid password']);
    }
} elseif ($action === 'validate') {
    $token = $body['token'] ?? '';
    echo json_encode(['valid' => hash_equals(generateToken(), $token)]);
} else {
    http_response_code(400);
    echo json_encode(['error' => 'Invalid action']);
}
