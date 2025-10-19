<?php
# save svg's and pdfs to files folder

if (!file_exists('files')) {
	mkdir('files', 0777, true);
}

if (!file_exists('files/certificates')) {
	mkdir('files/certificates', 0777, true);
}

if (!file_exists('files/qrs')) {
	mkdir('files/qrs', 0777, true);
}

if (!file_exists('files/svgs')) {
	mkdir('files/svgs', 0777, true);
}

if (!file_exists('files/signatures')) {
	mkdir('files/signatures', 0777, true);
}

# get the document content from the post request
$doc = $_POST['doc'] ?? '';
$filename = $_POST['filename'] ?? '';
$type = $_POST['type'] ?? 'qrs'; // certificates, qrs, svgs, or signatures

# validate inputs
if (empty($doc) || empty($filename)) {
	header('Content-Type: application/json');
	echo json_encode(['success' => false, 'error' => 'Missing doc or filename']);
	exit;
}

# save the document to the server with UTF-8 encoding
$filepath = 'files/' . $type . '/' . $filename;

// For JSON files, ensure UTF-8 encoding
if (pathinfo($filename, PATHINFO_EXTENSION) === 'json') {
	// Validate JSON before saving
	$decoded = json_decode($doc);
	if (json_last_error() !== JSON_ERROR_NONE) {
		header('Content-Type: application/json; charset=utf-8');
		echo json_encode(['success' => false, 'error' => 'Invalid JSON: ' . json_last_error_msg()]);
		exit;
	}
	// Re-encode with UTF-8 to ensure proper encoding
	$doc = json_encode($decoded, JSON_UNESCAPED_UNICODE | JSON_PRETTY_PRINT);
}

file_put_contents($filepath, $doc);

# return success
header('Content-Type: application/json');
echo json_encode(['success' => true]);
exit;
?>