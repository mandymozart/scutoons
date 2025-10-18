<?php

header('Content-Type: application/json');

// Get toon parameter
$toon = $_GET['toon'] ?? '';

$editions = json_decode(file_get_contents(__DIR__ . '/editions.json'), true);

if (!empty($toon)) {
    $editions = array_filter($editions, function ($edition) use ($toon) {
        return $edition['toon'] === $toon;
    });
}

# output json to http response
http_response_code(200);
echo json_encode($editions);
