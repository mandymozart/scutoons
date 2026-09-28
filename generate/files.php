<?php
require_once __DIR__ . '/auth-check.php';
# save svg's and pdfs to files folder

if (!file_exists('files')) {
	mkdir('files', 0777, true);
}

if (!file_exists('files/certificates')) {
	mkdir('files/certificates', 0777, true);
}

if (!file_exists('files/svgs')) {
	mkdir('files/svgs', 0777, true);
}

if (!file_exists('files/signatures')) {
	mkdir('files/signatures', 0777, true);
}

if (!file_exists('files/batch')) {
	mkdir('files/batch', 0777, true);
}

# get the document content from the post request
$doc = $_POST['doc'] ?? '';
$filename = $_POST['filename'] ?? '';
$type = $_POST['type'] ?? 'certificates';

# validate type against whitelist
$allowedTypes = ['certificates', 'svgs', 'signatures', 'batch', 'zips'];
if (!in_array($type, $allowedTypes)) {
	header('Content-Type: application/json');
	echo json_encode(['success' => false, 'error' => 'Invalid file type']);
	exit;
}

# validate inputs
if (empty($doc) || empty($filename)) {
	header('Content-Type: application/json');
	echo json_encode(['success' => false, 'error' => 'Missing doc or filename']);
	exit;
}

# save the document to the server with UTF-8 encoding
$filename = basename($filename);
if (!preg_match('/^[A-Za-z0-9_.-]{1,200}\.(pdf|svg|json|zip)$/', $filename) || str_starts_with($filename, '.')) {
	header('Content-Type: application/json');
	echo json_encode(['success' => false, 'error' => 'Invalid filename']);
	exit;
}
$filepath = __DIR__ . '/files/' . $type . '/' . $filename;

$ext = pathinfo($filename, PATHINFO_EXTENSION);

// PDFs arrive as data URIs (data:application/pdf;base64,...) — decode to binary
if ($ext === 'pdf') {
	if (str_starts_with($doc, 'data:')) {
		$doc = base64_decode(explode(',', $doc, 2)[1]);
	}
	if ($doc === false || strlen($doc) === 0) {
		header('Content-Type: application/json');
		echo json_encode(['success' => false, 'error' => 'Invalid PDF data']);
		exit;
	}
}

// For JSON files, ensure UTF-8 encoding
if ($ext === 'json') {
	$decoded = json_decode($doc);
	if (json_last_error() !== JSON_ERROR_NONE) {
		header('Content-Type: application/json; charset=utf-8');
		echo json_encode(['success' => false, 'error' => 'Invalid JSON: ' . json_last_error_msg()]);
		exit;
	}
	$doc = json_encode($decoded, JSON_UNESCAPED_UNICODE | JSON_UNESCAPED_SLASHES | JSON_PRETTY_PRINT);
}

file_put_contents($filepath, $doc);

# return success
header('Content-Type: application/json');
echo json_encode(['success' => true]);
exit;
?>