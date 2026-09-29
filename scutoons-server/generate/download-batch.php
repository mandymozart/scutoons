<?php
require_once __DIR__ . '/auth-check.php';
$filename = basename($_GET['file'] ?? '');

if (!preg_match('/^BATCH-[\w.-]+\.json$/', $filename)) {
    http_response_code(400);
    die('Invalid filename');
}

$manifestPath = 'files/batch/' . $filename;

if (!file_exists($manifestPath)) {
    http_response_code(404);
    die('Manifest not found');
}

$manifest = json_decode(file_get_contents($manifestPath), true);

if (!$manifest || !isset($manifest['codes'])) {
    http_response_code(500);
    die('Invalid manifest');
}

// Store ZIPs alongside the other batch output
if (!file_exists('files/zips')) {
    mkdir('files/zips', 0777, true);
}

$zipName = str_replace('.json', '.zip', $filename);
$zipPath = 'files/zips/' . $zipName;

$zip = new ZipArchive();
if ($zip->open($zipPath, ZipArchive::CREATE | ZipArchive::OVERWRITE) !== true) {
    http_response_code(500);
    die('Could not create ZIP archive');
}

// Include the manifest itself
$zip->addFile($manifestPath, 'manifest.json');

// Add files for each completed code
foreach ($manifest['codes'] as $item) {
    if ($item['status'] !== 'done') continue;

    $code = $item['code'];

    $files = [
        "files/svgs/SCUTOONS-QR-MEMBERSHIP-{$code}.svg" => "svgs/SCUTOONS-QR-MEMBERSHIP-{$code}.svg",
        "files/certificates/SCUTOONS-COA-{$code}.pdf"   => "certificates/SCUTOONS-COA-{$code}.pdf",
        "files/signatures/SIGNATURE-{$code}.json"       => "signatures/SIGNATURE-{$code}.json",
    ];

    foreach ($files as $src => $dest) {
        if (file_exists($src)) {
            $zip->addFile($src, $dest);
        }
    }
}

$zip->close();

header('Content-Type: application/zip');
header('Content-Disposition: attachment; filename="' . $zipName . '"');
header('Content-Length: ' . filesize($zipPath));
header('Cache-Control: no-store');
readfile($zipPath);
