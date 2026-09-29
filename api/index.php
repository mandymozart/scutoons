<?php
declare(strict_types=1);

/**
 * Accounts / items API (scutoons agents/ITEMS.md "HTTP API contract"). Every /api/ path that is not an existing
 * file or folder comes here (.htaccess on Apache, router.php for `php -S`). Existing folders keep their own
 * endpoints: /api/validate/, /api/items/, /api/editions/, /api/toons/, /api/keys/.
 *
 *   GET  /api/health/
 *   POST /api/auth/request/          { email, code?, item_id?, variant?, locale? } → { status: "sent" }
 *   POST /api/auth/verify/           { email, otp } | { token } → { status, session_token, user, created, claimed?, claim_error? }
 *   POST /api/auth/logout/
 *   GET  /api/me/                    { user, access, memberships, inventory, avatar }
 *   GET  /api/code/?c=CODE           public code info (never an email)
 *   GET|POST /api/access/            { code } or ?c= → device access (permissions of a code, access_token)
 *   POST /api/inventory/claim/       { code, item_id?, variant? }
 *   POST /api/inventory/craft/       { recipe_id }
 *   POST /api/inventory/grant/       { item_id, source, ref? }
 *   POST /api/inventory/import/      { items: [{ item_id, qty }] }
 *   GET|PUT /api/avatar/             { slots: { <slot>: entryId|null } }
 *   GET|PUT /api/progress/<toonId>/  { state, updated_at }
 *
 * Signed-in requests send `Authorization: Bearer <session_token>`.
 */
require_once __DIR__ . '/lib/bootstrap.php';

Http::run(function () {
    $method = $_SERVER['REQUEST_METHOD'] ?? 'GET';
    // /api/auth/request/ → auth/request (also below a sub folder)
    $path = (string) parse_url($_SERVER['REQUEST_URI'] ?? '/', PHP_URL_PATH);
    $path = preg_replace('#^.*?/api(?=/|$)#', '', $path, 1);
    $path = trim((string) $path, '/');
    $route = "$method $path";

    match (true) {
        $route === 'GET health' => Http::json(200, ['status' => 'ok', 'db' => Db::value('SELECT 1') == 1 ? 'ok' : 'error']),
        $route === 'POST auth/request' => Auth::request(Http::body()),
        $route === 'POST auth/verify' => Auth::verify(Http::body()),
        $route === 'POST auth/logout' => Auth::logout(),
        $route === 'GET me' => Inventory::me(),
        $route === 'GET code' => Codes::get(),
        $route === 'GET access', $route === 'POST access' => Codes::access(),
        $route === 'GET items' => Http::json(200, Catalog::data()),
        $route === 'POST inventory/claim' => Inventory::claim(Http::body()),
        $route === 'POST inventory/craft' => Inventory::craft(Http::body()),
        $route === 'POST inventory/grant' => Inventory::grant(Http::body()),
        $route === 'POST inventory/import' => Inventory::import(Http::body()),
        $route === 'GET avatar' => Avatar::get(),
        $route === 'PUT avatar' => Avatar::put(Http::body()),
        (bool) preg_match('#^(GET|PUT) progress/([A-Za-z0-9_-]{1,64})$#', $route, $m) =>
            $m[1] === 'GET' ? Progress::get($m[2]) : Progress::put($m[2], Http::body()),
        default => throw new ApiError(404, 'not_found', 'Unknown endpoint.'),
    };
});
