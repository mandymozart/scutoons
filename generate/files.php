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

# get the document content from the post request
$doc = $_POST['doc'] ?? '';
$filename = $_POST['filename'] ?? '';
$type = $_POST['type'] ?? 'qrs'; // certificate, qrs, or svgs

# validate inputs
if (empty($doc) || empty($filename)) {
	header('Content-Type: application/json');
	echo json_encode(['success' => false, 'error' => 'Missing doc or filename']);
	exit;
}

# save the document to the server
file_put_contents('files/' . $type . '/' . $filename, $doc);

# return success
header('Content-Type: application/json');
echo json_encode(['success' => true]);
exit;
?>