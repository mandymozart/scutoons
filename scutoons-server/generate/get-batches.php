<?php
require_once __DIR__ . '/auth-check.php';
header('Content-Type: application/json');

$dir = 'files/batch/';

if (!is_dir($dir)) {
    echo json_encode(['success' => true, 'batches' => []]);
    exit;
}

$files = glob($dir . 'BATCH-*.json');
$batches = [];

foreach ($files as $file) {
    $content = json_decode(file_get_contents($file), true);
    if (!$content || !isset($content['codes'])) continue;

    $done  = count(array_filter($content['codes'], fn($c) => $c['status'] === 'done'));
    $error = count(array_filter($content['codes'], fn($c) => $c['status'] === 'error'));

    $batches[] = [
        'filename'   => basename($file),
        'edition'    => $content['edition'] ?? '',
        'editionId'  => $content['editionId'] ?? '',
        'total'      => $content['total'] ?? 0,
        'done'       => $done,
        'errors'     => $error,
        'created_at' => $content['created_at'] ?? '',
        'complete'   => $done === ($content['total'] ?? -1),
    ];
}

usort($batches, fn($a, $b) => $b['created_at'] <=> $a['created_at']);

echo json_encode(['success' => true, 'batches' => $batches]);
