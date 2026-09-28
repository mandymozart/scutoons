<?php
declare(strict_types=1);

/** GET /api/items/ – the item catalog (public, ITEMS.md). Source: items.json next to this file. */
require_once __DIR__ . '/../lib/bootstrap.php';

Http::run(function () {
    if (($_SERVER['REQUEST_METHOD'] ?? 'GET') !== 'GET') throw new ApiError(405, 'method_not_allowed', 'Use GET.');
    // Public, identical for everyone: any origin may read it (the viewer, other toons' sites)
    header('Access-Control-Allow-Origin: ' . (Http::origin() !== null && Http::isAllowedOrigin((string) Http::origin()) ? Http::origin() : '*'));
    http_response_code(200);
    header('Content-Type: application/json; charset=utf-8');
    header('Cache-Control: public, max-age=300');
    echo json_encode(Catalog::data(), JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
    exit;
});
