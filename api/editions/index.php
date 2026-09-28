<?php
declare(strict_types=1);

/** GET /api/editions/?toon=<id> – public edition list (used by the generate SPA and the viewer inventory). */
require_once __DIR__ . '/../lib/bootstrap.php';

Http::run(function () {
    if (($_SERVER['REQUEST_METHOD'] ?? 'GET') !== 'GET') throw new ApiError(405, 'method_not_allowed', 'Use GET.');
    header('Access-Control-Allow-Origin: ' . (Http::origin() !== null && Http::isAllowedOrigin((string) Http::origin()) ? Http::origin() : '*'));

    $toon = $_GET['toon'] ?? '';
    $editions = json_decode(file_get_contents(__DIR__ . '/editions.json'), true);
    if ($toon !== '') {
        // array_values keeps it a JSON array after filtering
        $editions = array_values(array_filter($editions, fn ($edition) => $edition['toon'] === $toon));
    }

    http_response_code(200);
    header('Content-Type: application/json; charset=utf-8');
    header('Cache-Control: public, max-age=300');
    echo json_encode($editions, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
    exit;
});
