<?php
/**
 * Content store (api/lib/Content.php) end-to-end test against a running local server, plus the import script
 * against a copy of the local database.
 *
 *   php -S localhost:9000 router.php
 *   php scripts/migrate.php                       (content_versions, roles)
 *   php tests/content-smoke.php [http://localhost:9000] [path to scutoons viewer/static/api]
 *
 * Creates two throw-away users directly in the local DB (one member, one creator) with sessions, writes only to a
 * throw-away toon id (`smoke<random>`) and deletes its rows and the users at the end. Exit code 0 = all passed.
 */
if (PHP_SAPI !== 'cli') {
    http_response_code(404);
    exit;
}

require_once __DIR__ . '/../api/lib/bootstrap.php';
require_once __DIR__ . '/../api/lib/Content.php';

$base = rtrim($argv[1] ?? 'http://localhost:9000', '/');
$api = $base . '/api';
$staticApi = $argv[2] ?? (getenv('SCUTOONS_STATIC_API') ?: __DIR__ . '/../../scutoons/viewer/static/api');
$failures = 0;
$checks = 0;

function check(bool $ok, string $label, mixed $detail = null): void
{
    global $failures, $checks;
    $checks++;
    if ($ok) {
        echo "  PASS  $label\n";
        return;
    }
    $failures++;
    echo "  FAIL  $label" . ($detail !== null ? "\n        " . substr(is_string($detail) ? $detail : json_encode($detail, JSON_UNESCAPED_SLASHES), 0, 600) : '') . "\n";
}

/** @return array{status: int, body: mixed, raw: string, headers: array<string, string>} */
function call(string $method, string $url, mixed $body = null, ?string $token = null, array $headers = []): array
{
    $ch = curl_init($url);
    $h = array_merge(['Content-Type: application/json'], $headers);
    if ($token) $h[] = 'Authorization: Bearer ' . $token;
    $responseHeaders = [];
    curl_setopt_array($ch, [
        CURLOPT_CUSTOMREQUEST => $method,
        CURLOPT_RETURNTRANSFER => true,
        CURLOPT_HTTPHEADER => $h,
        CURLOPT_TIMEOUT => 20,
        CURLOPT_HEADERFUNCTION => function ($ch, $line) use (&$responseHeaders) {
            if (str_contains($line, ':')) {
                [$name, $value] = explode(':', $line, 2);
                $responseHeaders[strtolower(trim($name))] = trim($value);
            }
            return strlen($line);
        },
    ]);
    if ($body !== null) curl_setopt($ch, CURLOPT_POSTFIELDS, is_string($body) ? $body : json_encode($body, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE));
    $raw = (string) curl_exec($ch);
    $status = (int) curl_getinfo($ch, CURLINFO_RESPONSE_CODE);
    curl_close($ch);
    return ['status' => $status, 'body' => json_decode($raw, true) ?? $raw, 'raw' => $raw, 'headers' => $responseHeaders];
}

// ── Throw-away users + sessions (no mail round trip – the login flow has its own tests) ─────────────────
$run = gmdate('YmdHis') . bin2hex(random_bytes(2));
$toon = 'smoke' . strtolower($run);
[$member, $creator] = Db::transaction(fn() => [
    Users::create("content-smoke-$run-member@example.com", 'en'),
    Users::create("content-smoke-$run-creator@example.com", 'en'),
]);
Roles::grant($creator['id'], Roles::CREATOR, 'content-smoke');
$memberToken = Auth::createSession($member['id']);
$creatorToken = Auth::createSession($creator['id']);
echo "API $api · toon $toon\n\n";

$file = "$api/content/$toon/de/chapter1.json";
$v1 = ['title' => 'Eins', 'number' => 1, 'groups' => [], 'params' => new stdClass(), 'volume' => 0.5, 'text' => 'Größe „Ä“ / ok'];
$v2 = ['title' => 'Zwei', 'number' => 1, 'groups' => []];

try {
    echo "Roles\n";
    $r = call('GET', "$api/me/", null, $creatorToken);
    check($r['status'] === 200 && in_array('creator', $r['body']['user']['roles'] ?? [], true), '/me/ lists the creator role', $r['body']['user'] ?? $r);
    $r = call('PUT', $file, ['content' => $v1]);
    check($r['status'] === 401 && $r['body']['error'] === 'unauthorized', 'PUT without session → 401', $r);
    $r = call('PUT', $file, ['content' => $v1], $memberToken);
    check($r['status'] === 403 && $r['body']['error'] === 'forbidden', 'PUT as member → 403', $r);
    $r = call('GET', "$file?draft=1");
    check($r['status'] === 401, 'draft read without session → 401', $r);
    $r = call('GET', "$file?draft=1", null, $memberToken);
    check($r['status'] === 403, 'draft read as member → 403', $r);
    foreach (['versions?toon=' . $toon . '&locale=de&path=chapter1.json', 'index?toon=' . $toon] as $endpoint) {
        $r = call('GET', "$api/content/$endpoint", null, $memberToken);
        check($r['status'] === 403, "GET content/" . strtok($endpoint, '?') . " as member → 403", $r);
    }
    foreach (['publish', 'unpublish', 'restore'] as $action) {
        $r = call('POST', "$api/content/$action", ['toon' => $toon, 'locale' => 'de', 'path' => 'chapter1.json', 'id' => 1], $memberToken);
        check($r['status'] === 403, "POST content/$action as member → 403", $r);
        $r = call('POST', "$api/content/$action", ['toon' => $toon, 'locale' => 'de', 'path' => 'chapter1.json', 'id' => 1]);
        check($r['status'] === 401, "POST content/$action without session → 401", $r);
    }

    echo "\nDraft vs published\n";
    $r = call('GET', $file);
    check($r['status'] === 404, 'public GET before any version → 404', $r);
    $r = call('PUT', $file, ['content' => $v1, 'message' => 'first', 'base' => null], $creatorToken);
    check($r['status'] === 200 && $r['body']['version']['status'] === 'draft' && $r['body']['unchanged'] === false, 'PUT as creator → draft version', $r);
    $id1 = (int) ($r['body']['version']['id'] ?? 0);
    $r = call('PUT', $file, ['content' => $v1, 'base' => $id1], $creatorToken);
    check($r['status'] === 200 && $r['body']['unchanged'] === true && $r['body']['version']['id'] === $id1, 'PUT same content → unchanged, no new version', $r);
    $r = call('GET', $file);
    check($r['status'] === 404, 'draft is not public (404)', $r);
    $r = call('GET', "$file?draft=1", null, $creatorToken);
    check($r['status'] === 200 && ($r['body']['title'] ?? null) === 'Eins' && ($r['headers']['x-content-status'] ?? '') === 'draft', 'creator reads the draft', $r);
    check(str_contains($r['raw'], '"params":{}') && str_contains($r['raw'], '"volume":0.5') && str_contains($r['raw'], 'Größe „Ä“ / ok'),
        'stored JSON keeps {} objects and unescaped unicode/slashes', $r['raw']);
    $r = call('GET', "$file?draft=1&wrap=1", null, $creatorToken);
    check($r['status'] === 200 && $r['body']['version']['id'] === $id1 && $r['body']['published'] === null && ($r['body']['content']['title'] ?? '') === 'Eins',
        'wrapped read → version meta + content, nothing published', $r);

    $r = call('POST', "$api/content/publish", ['toon' => $toon, 'locale' => 'de', 'path' => 'chapter1.json'], $creatorToken);
    check($r['status'] === 200 && $r['body']['version']['id'] === $id1 && $r['body']['version']['status'] === 'published', 'publish latest → v1 published', $r);
    $r = call('GET', $file);
    check($r['status'] === 200 && ($r['body']['title'] ?? null) === 'Eins' && ($r['headers']['etag'] ?? '') === "\"v$id1\"", 'public GET → v1 with ETag', $r);
    $r = call('GET', $file, null, null, ['If-None-Match: "v' . $id1 . '"']);
    check($r['status'] === 304, 'If-None-Match → 304', $r['status']);

    $r = call('PUT', $file, ['content' => $v2, 'base' => $id1 - 1], $creatorToken);
    check($r['status'] === 409 && $r['body']['error'] === 'conflict' && $r['body']['latest']['id'] === $id1, 'PUT with stale base → 409 conflict', $r);
    $r = call('PUT', $file, ['content' => $v2, 'base' => $id1, 'message' => 'second'], $creatorToken);
    $id2 = (int) ($r['body']['version']['id'] ?? 0);
    check($r['status'] === 200 && $id2 > $id1, 'PUT v2 → new draft', $r);
    $r = call('GET', $file);
    check(($r['body']['title'] ?? null) === 'Eins', 'public still shows the published v1', $r);
    $r = call('GET', "$file?draft=1", null, $creatorToken);
    check(($r['body']['title'] ?? null) === 'Zwei', 'draft read shows v2', $r);
    $r = call('GET', "$file?version=$id1", null, $creatorToken);
    check(($r['body']['title'] ?? null) === 'Eins', '?version= reads an old version', $r);

    echo "\nVersions, restore, unpublish\n";
    $r = call('GET', "$api/content/versions?toon=$toon&locale=de&path=chapter1.json", null, $creatorToken);
    $versions = $r['body']['versions'] ?? [];
    check($r['status'] === 200 && count($versions) === 2 && $versions[0]['id'] === $id2 && $versions[0]['status'] === 'draft'
        && $versions[1]['status'] === 'published' && $versions[1]['message'] === 'first' && str_starts_with((string) $versions[0]['author_email'], 'content-smoke-'),
        'history: newest first, statuses, message, author email', $r);
    $r = call('POST', "$api/content/publish", ['toon' => $toon, 'locale' => 'de', 'path' => 'chapter1.json', 'version' => $id2], $creatorToken);
    check($r['status'] === 200, 'publish v2', $r);
    $r = call('GET', $file);
    check(($r['body']['title'] ?? null) === 'Zwei', 'public shows v2', $r);
    $count = (int) Db::value("SELECT COUNT(*) FROM content_versions WHERE toon = ? AND status = 'published'", [$toon]);
    check($count === 1, 'only one published version per file', $count);
    $r = call('POST', "$api/content/restore", ['id' => $id1], $creatorToken);
    $id3 = (int) ($r['body']['version']['id'] ?? 0);
    check($r['status'] === 200 && $id3 > $id2 && $r['body']['version']['status'] === 'draft' && $r['body']['version']['message'] === "restore #$id1", 'restore v1 → new draft v3', $r);
    $r = call('GET', $file);
    check(($r['body']['title'] ?? null) === 'Zwei', 'restore does not change what is public', $r);
    $r = call('POST', "$api/content/publish", ['toon' => $toon, 'locale' => 'de', 'path' => 'chapter1.json'], $creatorToken);
    $r = call('GET', $file);
    check(($r['body']['title'] ?? null) === 'Eins' && ($r['headers']['etag'] ?? '') === "\"v$id3\"", 'publish restored → public shows v1 content again', $r);
    $r = call('POST', "$api/content/publish", ['toon' => $toon, 'locale' => 'en', 'path' => 'chapter1.json', 'version' => $id3], $creatorToken);
    check($r['status'] === 404, 'publish a version under another file → 404', $r);
    $r = call('GET', "$api/content/index?toon=$toon", null, $creatorToken);
    check($r['status'] === 200 && count($r['body']['files']) === 1 && $r['body']['files'][0]['published'] === $id3 && $r['body']['files'][0]['versions'] === 3, 'index lists the file', $r);
    $r = call('POST', "$api/content/unpublish", ['toon' => $toon, 'locale' => 'de', 'path' => 'chapter1.json'], $creatorToken);
    check($r['status'] === 200 && $r['body']['unpublished'] === 1, 'unpublish', $r);
    $r = call('GET', $file);
    check($r['status'] === 404, 'public GET after unpublish → 404', $r);

    echo "\nPath whitelist and body validation\n";
    $ok = call('PUT', "$api/content/$toon/de/chapter2/scene-14.json", ['content' => ['label' => 'scene-14']], $creatorToken);
    check($ok['status'] === 200, 'sub folder path chapter2/scene-14.json allowed', $ok);
    $badPaths = [
        "$toon/de/Chapter1.json" => 400, "$toon/de/chapter1.txt" => 400, "$toon/de/a/b/c.json" => 400,
        "$toon/de/chapter%201.json" => 400, "$toon/de/.json" => [400, 404], "$toon/xx-yy/chapter1.json" => 400,
        "Bad/de/chapter1.json" => 400, "$toon/de" => 404, "$toon/de/..%2Fsecret.json" => [400, 404],
    ];
    // dot paths are already refused by the router / .htaccess (404) before they reach Content
    foreach ($badPaths as $path => $expected) {
        $r = call('PUT', "$api/content/$path", ['content' => ['x' => 1]], $creatorToken);
        $expected = (array) $expected;
        check(in_array($r['status'], $expected, true), "PUT content/$path → " . implode('|', $expected), $r);
    }
    $r = call('GET', "$api/content/$toon/de/chapter1.JSON");
    check($r['status'] === 400, 'public GET with an invalid path → 400', $r);
    $r = call('PUT', $file, '{not json', $creatorToken);
    check($r['status'] === 400 && $r['body']['error'] === 'invalid_json', 'invalid JSON body → 400', $r);
    $r = call('PUT', $file, ['message' => 'no content'], $creatorToken);
    check($r['status'] === 400 && $r['body']['error'] === 'missing_content', 'missing content → 400', $r);
    $r = call('PUT', $file, ['content' => 'a string'], $creatorToken);
    check($r['status'] === 422 && $r['body']['error'] === 'invalid_content', 'content that is not an object/array → 422', $r);
    $r = call('PUT', $file, ['content' => ['blob' => str_repeat('x', Content::maxBytes() + 10)]], $creatorToken);
    check($r['status'] === 413, 'content over the size limit → 413', $r['status']);

    echo "\nImport script (against a copy of the database)\n";
    $dbFile = defined('DB_SQLITE_PATH') ? DB_SQLITE_PATH : null;
    if ($dbFile === null || !is_file($dbFile) || !is_dir($staticApi)) {
        check(false, 'import test needs the local SQLite DB and the viewer static api folder', [$dbFile, $staticApi]);
    } else {
        $copy = sys_get_temp_dir() . "/content-smoke-$run.sqlite";
        copy($dbFile, $copy);
        $pdo = new PDO('sqlite:' . $copy, null, null, [PDO::ATTR_ERRMODE => PDO::ERRMODE_EXCEPTION, PDO::ATTR_DEFAULT_FETCH_MODE => PDO::FETCH_ASSOC]);
        $pdo->exec("DELETE FROM content_versions WHERE toon = 'catpope'");
        $pdo = null;
        $script = escapeshellarg(PHP_BINARY) . ' ' . escapeshellarg(__DIR__ . '/../scripts/import-content.php') . ' ' . escapeshellarg($staticApi) . ' --db=' . escapeshellarg($copy);
        $before = (int) Db::value('SELECT COUNT(*) FROM content_versions');
        exec("$script --toon=catpope 2>&1", $out1, $code1);
        exec("$script --toon=catpope 2>&1", $out2, $code2);
        $pdo = new PDO('sqlite:' . $copy, null, null, [PDO::ATTR_ERRMODE => PDO::ERRMODE_EXCEPTION, PDO::ATTR_DEFAULT_FETCH_MODE => PDO::FETCH_ASSOC]);
        $rows = $pdo->query("SELECT locale, path, status, json, author FROM content_versions WHERE toon = 'catpope'")->fetchAll();
        $expected = 0;
        foreach (glob("$staticApi/catpope/*/{toon,season*,chapter*}.json", GLOB_BRACE) ?: [] as $f) $expected++;
        foreach (glob("$staticApi/catpope/*/chapter*/*.json") ?: [] as $f) $expected++;
        check($code1 === 0 && count($rows) === $expected && $expected > 10, "import: $expected files as versions", [$code1, count($rows), $expected, array_slice($out1, -2)]);
        check(count(array_filter($rows, fn($r) => $r['status'] === 'published' && $r['author'] === null)) === count($rows), 'import: all published, no author');
        check($code2 === 0 && str_contains(implode("\n", $out2), '0 imported') && (int) $pdo->query("SELECT COUNT(*) FROM content_versions WHERE toon = 'catpope'")->fetchColumn() === $expected,
            'import twice → nothing new (idempotent)', array_slice($out2, -1));
        $chapter = null;
        foreach ($rows as $row) if ($row['locale'] === 'de' && $row['path'] === 'chapter1.json') $chapter = $row;
        $static = json_decode((string) file_get_contents("$staticApi/catpope/de/chapter1.json"), true);
        check($chapter !== null && json_decode($chapter['json'], true) === $static, 'import: stored chapter1 (de) equals the static file');
        $scene = array_filter($rows, fn($r) => $r['path'] === 'chapter2/scene-14.json');
        check(count($scene) === 2, 'import: VN scene files of both locales', count($scene));
        check((int) Db::value('SELECT COUNT(*) FROM content_versions') === $before, 'import with --db leaves the configured database alone');
        $pdo = null;
        @unlink($copy);
    }
} finally {
    Db::run('DELETE FROM content_versions WHERE toon = ?', [$toon]);
    foreach ([$member, $creator] as $user) {
        Db::run('DELETE FROM sessions WHERE user_id = ?', [$user['id']]);
        Db::run('DELETE FROM user_roles WHERE user_id = ?', [$user['id']]);
        Db::run('DELETE FROM inventory WHERE user_id = ?', [$user['id']]);
        Db::run('DELETE FROM avatar_slots WHERE user_id = ?', [$user['id']]);
        Db::run('DELETE FROM users WHERE id = ?', [$user['id']]);
    }
}

echo "\n" . ($checks - $failures) . "/$checks checks passed\n";
exit($failures === 0 ? 0 : 1);
