<?php
// Included at the top of every protected generate endpoint.
// Reads X-Generate-Token header and validates against the configured password.
if (!defined('GENERATE_PASSWORD')) require_once __DIR__ . '/../config.php';

$token = $_SERVER['HTTP_X_GENERATE_TOKEN'] ?? $_GET['token'] ?? '';
$expected = hash_hmac('sha256', 'generate-access', GENERATE_PASSWORD);

if (!hash_equals($expected, $token)) {
    http_response_code(401);
    header('Content-Type: application/json');
    echo json_encode(['error' => 'Unauthorized']);
    exit;
}
