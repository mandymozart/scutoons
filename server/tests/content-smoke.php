<?php
/**
 * The content store by entity key (src/Domain/Content/EntityStore.php, /api/entities – the creator's write path)
 * end-to-end against a running local server, plus import-content against a copy of the local database.
 *
 *   php tests/content-smoke.php [http://localhost:9000] [interchange folder, default ../shared/content/seed]
 *
 * Creates two throw-away users directly in the local DB (one member, one creator) with sessions, writes only to a
 * throw-away toon id (`smoke<random>`) and deletes its rows and the users at the end. Exit code 0 = all passed.
 */
if (PHP_SAPI !== 'cli') {
    http_response_code(404);
    exit;
}

require_once __DIR__ . '/support/legacy.php';

$base = rtrim($argv[1] ?? 'http://localhost:9000', '/');
$api = $base . '/api';
$seedRoot = $argv[2] ?? (getenv('SCUTOONS_CONTENT_SEED') ?: __DIR__ . '/../../shared/content/seed');
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

$key = "chapter:$toon:1:de";
$entity = "$api/entities/$key";
$v1 = ['title' => 'Eins', 'number' => 1, 'groups' => [], 'params' => new stdClass(), 'volume' => 0.5, 'text' => 'Größe „Ä“ / ok'];
$v2 = ['title' => 'Zwei', 'number' => 1, 'groups' => []];

try {
    echo "Roles (everything needs the creator role)\n";
    $r = call('PUT', $entity, ['content' => $v1]);
    check($r['status'] === 401 && $r['body']['error'] === 'unauthorized', 'PUT without session → 401', $r);
    $r = call('PUT', $entity, ['content' => $v1], $memberToken);
    check($r['status'] === 403 && $r['body']['error'] === 'forbidden', 'PUT as member → 403', $r);
    foreach (['', '?draft=1', '/versions'] as $suffix) {
        check(call('GET', "$entity$suffix")['status'] === 401, "GET entities/<key>$suffix without session → 401");
        check(call('GET', "$entity$suffix", null, $memberToken)['status'] === 403, "GET entities/<key>$suffix as member → 403");
    }
    check(call('GET', "$api/entities?toon=$toon", null, $memberToken)['status'] === 403, 'GET entities?toon= as member → 403');
    foreach (['publish', 'unpublish', 'restore'] as $action) {
        check(call('POST', "$entity/$action", ['id' => 1], $memberToken)['status'] === 403, "POST <key>/$action as member → 403");
        check(call('POST', "$entity/$action", ['id' => 1])['status'] === 401, "POST <key>/$action without session → 401");
    }

    echo "\nDraft vs published\n";
    $r = call('GET', $entity, null, $creatorToken);
    check($r['status'] === 404, 'published read before any version → 404', $r);
    $r = call('PUT', $entity, ['content' => $v1, 'message' => 'first', 'base' => null], $creatorToken);
    check($r['status'] === 200 && $r['body']['version']['status'] === 'draft' && $r['body']['unchanged'] === false && $r['body']['version']['key'] === $key,
        'PUT as creator → draft version with its key', $r);
    $id1 = (int) ($r['body']['version']['id'] ?? 0);
    $r = call('PUT', $entity, ['content' => $v1, 'base' => $id1], $creatorToken);
    check($r['status'] === 200 && $r['body']['unchanged'] === true && $r['body']['version']['id'] === $id1, 'PUT same content → unchanged, no new version', $r);
    check(call('GET', $entity, null, $creatorToken)['status'] === 404, 'a draft is not published (404)');
    $r = call('GET', "$entity?draft=1", null, $creatorToken);
    check($r['status'] === 200 && ($r['body']['title'] ?? null) === 'Eins' && ($r['headers']['x-content-status'] ?? '') === 'draft', 'creator reads the draft', $r);
    check(str_contains($r['raw'], '"params":{}') && str_contains($r['raw'], '"volume":0.5') && str_contains($r['raw'], 'Größe „Ä“ / ok'),
        'stored JSON keeps {} objects and unescaped unicode/slashes', $r['raw']);
    $r = call('GET', "$entity?draft=1&wrap=1", null, $creatorToken);
    check($r['status'] === 200 && $r['body']['version']['id'] === $id1 && $r['body']['published'] === null && ($r['body']['content']['title'] ?? '') === 'Eins',
        'wrapped read → version meta + content, nothing published', $r);
    $r = call('POST', "$entity/publish", null, $creatorToken);
    check($r['status'] === 200 && $r['body']['version']['id'] === $id1 && $r['body']['version']['status'] === 'published', 'publish (no body) → newest published', $r);
    $r = call('GET', $entity, null, $creatorToken);
    check($r['status'] === 200 && ($r['body']['title'] ?? null) === 'Eins' && ($r['headers']['x-content-version'] ?? '') === (string) $id1, 'published read → v1', $r);

    $r = call('PUT', $entity, ['content' => $v2, 'base' => $id1 - 1], $creatorToken);
    check($r['status'] === 409 && $r['body']['error'] === 'conflict' && $r['body']['latest']['id'] === $id1, 'PUT with stale base → 409 conflict', $r);
    $r = call('PUT', $entity, ['content' => $v2, 'base' => $id1, 'message' => 'second'], $creatorToken);
    $id2 = (int) ($r['body']['version']['id'] ?? 0);
    check($r['status'] === 200 && $id2 > $id1, 'PUT v2 → new draft', $r);
    check((call('GET', $entity, null, $creatorToken)['body']['title'] ?? null) === 'Eins', 'published still v1');
    check((call('GET', "$entity?draft=1", null, $creatorToken)['body']['title'] ?? null) === 'Zwei', 'draft read shows v2');
    check((call('GET', "$entity?version=$id1", null, $creatorToken)['body']['title'] ?? null) === 'Eins', '?version= reads an old version');

    echo "\nVersions, restore, unpublish, index\n";
    $r = call('GET', "$entity/versions", null, $creatorToken);
    $versions = $r['body']['versions'] ?? [];
    check($r['status'] === 200 && count($versions) === 2 && $versions[0]['id'] === $id2 && $versions[0]['status'] === 'draft'
        && $versions[1]['status'] === 'published' && $versions[1]['message'] === 'first' && str_starts_with((string) $versions[0]['author_email'], 'content-smoke-'),
        'history: newest first, statuses, message, author email', $r);
    check(call('POST', "$entity/publish", ['version' => $id2], $creatorToken)['status'] === 200, 'publish v2');
    $count = (int) Db::value("SELECT COUNT(*) FROM entity_versions WHERE entity_key = ? AND status = 'published'", [$key]);
    check($count === 1, 'only one published version per entity', $count);
    $r = call('POST', "$entity/restore", ['id' => $id1], $creatorToken);
    $id3 = (int) ($r['body']['version']['id'] ?? 0);
    check($r['status'] === 200 && $id3 > $id2 && $r['body']['version']['status'] === 'draft' && $r['body']['version']['message'] === "restore #$id1", 'restore v1 → new draft v3', $r);
    check((call('GET', $entity, null, $creatorToken)['body']['title'] ?? null) === 'Zwei', 'restore does not change what is published');
    call('POST', "$entity/publish", null, $creatorToken);
    check((call('GET', $entity, null, $creatorToken)['body']['title'] ?? null) === 'Eins', 'publish restored → v1 content again');
    $r = call('POST', "$api/entities/chapter:$toon:1:en/publish", ['version' => $id3], $creatorToken);
    check($r['status'] === 404, 'publish a version under another key → 404', $r);
    $r = call('POST', "$api/entities/chapter:$toon:2:de/restore", ['id' => $id1], $creatorToken);
    check($r['status'] === 404, 'restore a version under another key → 404', $r);
    $r = call('GET', "$api/entities?toon=$toon", null, $creatorToken);
    $listed = $r['body']['entities'] ?? [];
    check($r['status'] === 200 && count($listed) === 1 && $listed[0]['key'] === $key && $listed[0]['kind'] === 'chapter' && $listed[0]['locale'] === 'de'
        && $listed[0]['published'] === $id3 && $listed[0]['versions'] === 3, 'index lists the entity', $r);
    $r = call('POST', "$entity/unpublish", null, $creatorToken);
    check($r['status'] === 200 && $r['body']['unpublished'] === 1, 'unpublish', $r);
    check(call('GET', $entity, null, $creatorToken)['status'] === 404, 'published read after unpublish → 404');

    echo "\nKeys and bodies\n";
    $ok = call('PUT', "$api/entities/scene:$toon:2:scene-14:de", ['content' => ['label' => 'scene-14']], $creatorToken);
    check($ok['status'] === 200, 'scene key scene:<toon>:2:scene-14:de', $ok);
    // chapter 0 is a season's prologue (a valid key); season 0 and leading zeros are not
    $ok = call('PUT', "$api/entities/chapter:$toon:0:de", ['content' => ['title' => 'Prologue', 'number' => 0]], $creatorToken);
    check($ok['status'] === 200, 'chapter key chapter:<toon>:0:de (prologue)', $ok);
    foreach (["chapter:$toon:1", "chapter:$toon:00:de", "season:$toon:0:de", "Chapter:$toon:1:de", "toon:scutoons", "unknown:$toon", "season:$toon:1:deu", 'characters:x'] as $bad) {
        $r = call('PUT', "$api/entities/$bad", ['content' => ['x' => 1]], $creatorToken);
        check(in_array($r['status'], [400, 404], true), "PUT entities/$bad → 400|404", $r);
    }
    check(call('GET', "$api/entities/nope", null, $creatorToken)['body']['error'] === 'invalid_key', 'unknown key → 400 invalid_key');
    check(call('GET', "$api/content/$toon/de/chapter1.json")['status'] === 404, 'the old path API is gone (404)');
    $r = call('PUT', $entity, '{not json', $creatorToken);
    check($r['status'] === 400 && $r['body']['error'] === 'invalid_json', 'invalid JSON body → 400', $r);
    $r = call('PUT', $entity, ['message' => 'no content'], $creatorToken);
    check($r['status'] === 400 && $r['body']['error'] === 'missing_content', 'missing content → 400', $r);
    $r = call('PUT', $entity, ['content' => 'a string'], $creatorToken);
    check($r['status'] === 422 && $r['body']['error'] === 'invalid_content', 'content that is not an object/array → 422', $r);
    $r = call('PUT', $entity, ['content' => ['blob' => str_repeat('x', Content::maxBytes() + 10)]], $creatorToken);
    check($r['status'] === 413, 'content over the size limit → 413', $r['status']);

    echo "\nImport (against a copy of the database)\n";
    $dbFile = defined('DB_SQLITE_PATH') ? DB_SQLITE_PATH : null;
    if ($dbFile === null || !is_file($dbFile) || !is_dir("$seedRoot/catpope")) {
        check(false, 'import test needs the local SQLite DB and the interchange folder', [$dbFile, $seedRoot]);
    } else {
        $copy = sys_get_temp_dir() . "/content-smoke-$run.sqlite";
        copy($dbFile, $copy);
        $pdo = new PDO('sqlite:' . $copy, null, null, [PDO::ATTR_ERRMODE => PDO::ERRMODE_EXCEPTION, PDO::ATTR_DEFAULT_FETCH_MODE => PDO::FETCH_ASSOC]);
        $pdo->exec("DELETE FROM entity_versions WHERE toon = 'catpope'");
        $pdo = null;
        $script = escapeshellarg(PHP_BINARY) . ' ' . escapeshellarg(__DIR__ . '/../bin/console') . ' import-content ' . escapeshellarg($seedRoot) . ' --db=' . escapeshellarg($copy);
        $before = (int) Db::value('SELECT COUNT(*) FROM entity_versions');
        exec("$script --toon=catpope 2>&1", $out1, $code1);
        exec("$script --toon=catpope 2>&1", $out2, $code2);
        $pdo = new PDO('sqlite:' . $copy, null, null, [PDO::ATTR_ERRMODE => PDO::ERRMODE_EXCEPTION, PDO::ATTR_DEFAULT_FETCH_MODE => PDO::FETCH_ASSOC]);
        $rows = $pdo->query("SELECT entity_key, status, json, author FROM entity_versions WHERE toon = 'catpope'")->fetchAll();
        $expected = 1; // toon:catpope from toons.json
        foreach (glob("$seedRoot/catpope/*/{toon,season*,chapter*,locations,routes}.json", GLOB_BRACE) ?: [] as $f) $expected++;
        foreach (glob("$seedRoot/catpope/*/chapter*/*.json") ?: [] as $f) $expected++;
        foreach (['cast', 'credits'] as $name) if (is_file("$seedRoot/catpope/_/$name.json")) $expected++;
        check($code1 === 0 && count($rows) === $expected && $expected > 10, "import: $expected entities as versions", [$code1, count($rows), $expected, array_slice($out1, -2)]);
        check(count(array_filter($rows, fn($r) => $r['status'] === 'published' && $r['author'] === null)) === count($rows), 'import: all published, no author');
        check($code2 === 0 && str_contains(implode("\n", $out2), '0 imported') && (int) $pdo->query("SELECT COUNT(*) FROM entity_versions WHERE toon = 'catpope'")->fetchColumn() === $expected,
            'import twice → nothing new (idempotent)', array_slice($out2, -1));
        $chapter = null;
        foreach ($rows as $row) if ($row['entity_key'] === 'chapter:catpope:1:de') $chapter = $row;
        $file = json_decode((string) file_get_contents("$seedRoot/catpope/de/chapter1.json"), true);
        check($chapter !== null && json_decode($chapter['json'], true) === $file, 'import: chapter:catpope:1:de equals its file');
        check(count(array_filter($rows, fn($r) => str_starts_with($r['entity_key'], 'scene:catpope:2:'))) >= 2, 'import: VN scenes of chapter 2 as scene keys');
        check((int) Db::value('SELECT COUNT(*) FROM entity_versions') === $before, 'import with --db leaves the configured database alone');

        // --sql (production seed for phpMyAdmin): one guarded INSERT per entity, literals decode back to the same JSON
        exec("$script --toon=catpope --sql 2>" . (PHP_OS_FAMILY === 'Windows' ? 'NUL' : '/dev/null'), $sqlOut, $sqlCode);
        $sql = implode("\n", $sqlOut);
        preg_match_all("/^SELECT '([a-z0-9:_-]+)', '([a-z-]+)', '(?:catpope)', (?:'[a-z]{2}'|NULL), '((?:[^'\\\\]++|\\\\.)*+)', NULL/m", $sql, $inserts, PREG_SET_ORDER);
        $roundTrip = true;
        foreach ($inserts as [, $insertKey, , $literal]) {
            $json = preg_replace_callback('/\\\\(.)/s', fn($m) => ['0' => "\0", 'n' => "\n", 'r' => "\r", 'Z' => "\x1a"][$m[1]] ?? $m[1], $literal);
            $stored = null;
            foreach ($rows as $row) if ($row['entity_key'] === $insertKey) $stored = json_decode($row['json'], true);
            if (json_decode($json, true) !== $stored) $roundTrip = false;
        }
        check($sqlCode === 0 && count($inserts) === $expected && substr_count($sql, 'WHERE NOT EXISTS') === $expected && $roundTrip,
            '--sql: guarded INSERT per entity, MySQL literals decode to the imported JSON', [$sqlCode, count($inserts), $expected, $roundTrip]);
        check((int) Db::value('SELECT COUNT(*) FROM entity_versions') === $before, '--sql writes nothing');
        $pdo = null;
        @unlink($copy);
    }
} finally {
    Db::run('DELETE FROM entity_versions WHERE toon = ?', [$toon]);
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
