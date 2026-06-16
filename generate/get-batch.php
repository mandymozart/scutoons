<?php
header('Content-Type: application/json');

$filename = basename($_GET['file'] ?? '');

if (!preg_match('/^BATCH-[\w.-]+\.json$/', $filename)) {
    http_response_code(400);
    echo json_encode(['success' => false, 'message' => 'Invalid filename']);
    exit;
}

$filepath = 'files/batch/' . $filename;

if (!file_exists($filepath)) {
    http_response_code(404);
    echo json_encode(['success' => false, 'message' => 'Manifest not found']);
    exit;
}

echo file_get_contents($filepath);
