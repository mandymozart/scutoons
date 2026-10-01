<?php
/**
 * Contract suite of the library endpoints (/api/library/*, /api/events; agents/DATA.md §3, docs/API.md "Library")
 * against a running server – run by tests/run-contract.php on a copy of the local database.
 *
 *   php tests/entities-smoke.php [http://localhost:9000]
 *
 * Needs a server with CONTENT_SHOW_DRAFTS off. Creates a throw-away creator + member directly in the DB, writes a
 * throw-away toon (`ent<random>`) through the content API (the creator's write path), checks the composed answers,
 * gates, locale fallback, ETag/304, draft mode and that a publish shows up at once (cache keyed by version ids), then
 * deletes its rows and the users. Exit code 0 = all passed.
 */
if (PHP_SAPI !== 'cli') {
    http_response_code(404);
    exit;
}

require_once __DIR__ . '/support/legacy.php';

$base = rtrim($argv[1] ?? 'http://localhost:9000', '/');
$api = $base . '/api';
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
    if ($body !== null) curl_setopt($ch, CURLOPT_POSTFIELDS, json_encode($body, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE));
    $raw = (string) curl_exec($ch);
    $status = (int) curl_getinfo($ch, CURLINFO_RESPONSE_CODE);
    curl_close($ch);
    return ['status' => $status, 'body' => json_decode($raw, true) ?? $raw, 'raw' => $raw, 'headers' => $responseHeaders];
}

$run = gmdate('YmdHis') . bin2hex(random_bytes(2));
$toon = 'ent' . strtolower($run);
[$member, $creator] = Db::transaction(fn() => [
    Users::create("entities-smoke-$run-member@example.com", 'en'),
    Users::create("entities-smoke-$run-creator@example.com", 'en'),
]);
Roles::grant($creator['id'], Roles::CREATOR, 'entities-smoke');
$memberToken = Auth::createSession($member['id']);
$creatorToken = Auth::createSession($creator['id']);
echo "API $api · toon $toon\n\n";

/**
 * Saves a draft through the creator's write path (/api/entities) and publishes it (unless $publish is false). The key
 * is given without the toon: `chapter:1:de` → `chapter:<toon>:1:de`; platform-wide keys stay as they are.
 */
function store(string $key, mixed $content, bool $publish = true): void
{
    global $api, $toon, $creatorToken;
    $parts = explode(':', $key);
    $full = in_array($parts[0], ['characters', 'moods', 'asset-requests', 'people', 'roles', 'events', 'map'], true) ? $key : $parts[0] . ":$toon" . (count($parts) > 1 ? ':' . implode(':', array_slice($parts, 1)) : '');
    $r = call('PUT', "$api/entities/$full", ['content' => $content], $creatorToken);
    if ($r['status'] !== 200) throw new RuntimeException("PUT $full: " . $r['raw']);
    if ($publish) {
        $r = call('POST', "$api/entities/$full/publish", null, $creatorToken);
        if ($r['status'] !== 200) throw new RuntimeException("publish $full: " . $r['raw']);
    }
}

$lib = "$api/library/toons/$toon";
try {
    echo "Before the toon exists\n";
    $r = call('GET', $lib);
    check($r['status'] === 404 && $r['body']['error'] === 'not_found', 'unknown toon → 404 not_found', $r);
    $r = call('GET', "$api/library/toons");
    check($r['status'] === 200 && is_array($r['body']) && !in_array($toon, array_column($r['body'], 'id'), true), 'toons list is a JSON array without it', $r['raw']);

    // the creator's write path: index entry, toon, season (1 published, 2 draft, 3 coming soon), chapters, a scene
    store('toon', ['id' => $toon, 'uri' => $toon, 'status' => 'published', 'sourceLanguage' => 'de', 'languages' => ['de', 'en']]);
    foreach (['de', 'en'] as $l) {
        store("toon-settings:$l", ['id' => $toon, 'title' => "Toon $l", 'status' => 'published']);
        store("season:1:$l", ['title' => "Staffel $l", 'sequence' => [
            ['type' => 'chapter', 'number' => 1, 'status' => 'published'],
            ['type' => 'chapter', 'number' => 2, 'status' => 'draft'],
            ['type' => 'chapter', 'number' => 3, 'status' => 'coming-soon'],
        ]]);
        store("chapter:2:$l", ['title' => "Zwei $l", 'number' => 2, 'groups' => []]);
    }
    store('chapter:1:de', ['title' => 'Eins', 'number' => 1, 'thumbnail' => 't.webp', 'audio' => ['volume' => 0.5],
        'groups' => [['ref' => ['src' => 'chapter1/scene-a.json']], ['label' => 'inline', 'slices' => []]]]);
    store('scene:1:scene-a:de', ['label' => 'scene-a', 'params' => new stdClass(), 'events' => []]);
    store('chapter:3:de', ['title' => 'Drei', 'number' => 3, 'release' => '2027-01-01', 'groups' => [['label' => 'secret']]]);
    store('locations:de', [['id' => 'cathedral']]);

    echo "\nToons list + toon bundle\n";
    $r = call('GET', "$api/library/toons", null, null, ['Origin: https://elsewhere.example']);
    $entry = array_values(array_filter(is_array($r['body']) ? $r['body'] : [], fn($t) => ($t['id'] ?? '') === $toon))[0] ?? null;
    check($r['status'] === 200 && $entry !== null && $entry['languages'] === ['de', 'en'], 'toons list has the published index entry', $r['raw']);
    check(($r['headers']['access-control-allow-origin'] ?? '') === '*' && ($r['headers']['cache-control'] ?? '') === 'public, max-age=60', 'public: any origin, cached 60 s', $r['headers']);
    $r = call('GET', "$lib?locale=en");
    check($r['status'] === 200 && array_keys($r['body']) === ['entry', 'toon', 'seasons', 'locations', 'routes'], 'toon bundle shape', $r['raw']);
    check(($r['body']['toon']['title'] ?? '') === 'Toon en' && ($r['headers']['content-language'] ?? '') === 'en', 'toon.json in the asked locale', $r['headers']);
    // the old per-toon map documents are read only while no world map (entity `map`, agents/MAP.md) is stored
    $worldBefore = call('GET', "$api/entities/map?wrap=1", null, $creatorToken);
    if ($worldBefore['status'] === 404) check(($r['body']['locations'] ?? null) === [['id' => 'cathedral']], 'locations: en missing → de (source)', $r['body']['locations'] ?? null);
    else check(($r['body']['locations'] ?? null) === [], 'locations: computed from the world map (the old document is no longer read)', $r['body']['locations'] ?? null);
    $chapters = $r['body']['seasons'][0]['chapters'] ?? [];
    check(array_column($chapters, 'number') === [1, 3] && array_column($chapters, 'status') === ['published', 'coming-soon'], 'season overview lists published + coming-soon, not the draft', $chapters);
    check(isset($chapters[0]) && !array_key_exists('groups', $chapters[0]) && !array_key_exists('audio', $chapters[0]) && $chapters[0]['title'] === 'Eins',
        'chapter summaries carry no groups/audio', $chapters[0] ?? null);
    echo "\nWorld map (/library/map, agents/MAP.md)\n";
    // a world map with a place of this toon, a hidden one, one of an unknown (unlisted) toon; restored at the end
    $previousMap = $worldBefore['status'] === 200 ? ($worldBefore['body']['published']['id'] ?? null) : null;
    $world = ['width' => 2048, 'height' => 1046, 'layers' => [['id' => 'terrain', 'image' => 'catpope/images/map/map-layer-terrain.svg']],
        'locations' => [
            ['id' => "$toon-home", 'name' => ['de' => 'Zuhause', 'en' => 'Home'], 'x' => 100, 'y' => 200, 'toons' => [$toon], 'chapter' => ['toon' => $toon, 'number' => 1]],
            ['id' => "$toon-tower", 'name' => ['de' => 'Turm'], 'x' => 300, 'y' => 400, 'toons' => [$toon], 'hidden' => true, 'unlock' => ['toon' => $toon, 'chapter' => 1, 'at' => 0.5]],
            ['id' => "$toon-secret", 'name' => ['de' => 'Geheim'], 'x' => 5, 'y' => 5, 'toons' => ["x$toon"]],
        ],
        'routes' => [['id' => "$toon-way", 'name' => ['de' => 'Weg', 'en' => 'Way'], 'toons' => [$toon], 'style' => 'trail',
            'waypoints' => [['location' => "$toon-home"], ['id' => 'bridge', 'x' => 200, 'y' => 300, 'name' => ['de' => 'Brücke', 'en' => 'Bridge']], ['location' => "$toon-tower"]]]],
    ];
    store('map', $world);
    $r = call('GET', "$api/library/map?locale=en&toon=$toon", null, null, ['Origin: https://elsewhere.example']);
    $mine = array_values(array_filter($r['body']['locations'] ?? [], fn($p) => str_starts_with($p['id'] ?? '', $toon)));
    check($r['status'] === 200 && array_keys($r['body']) === ['width', 'height', 'layers', 'locations', 'routes', 'view'], '/library/map answer shape (+ view with ?toon=)', $r['raw']);
    check(array_column($mine, 'id') === ["$toon-home", "$toon-tower"], "only listed toons' places (the unknown toon's place stays out)", array_column($mine, 'id'));
    check(array_column($mine, 'name') === ['Home', 'Turm'] && ($r['headers']['x-content-fallback'] ?? '') === '1', 'names resolved to en, missing → de + X-Content-Fallback', [$mine, $r['headers']]);
    check(($mine[1]['hidden'] ?? null) === true && ($mine[1]['unlock']['at'] ?? null) === 0.5, 'hidden places are sent with their unlock point (the state is the reader\'s)', $mine[1] ?? null);
    $route = array_values(array_filter($r['body']['routes'] ?? [], fn($x) => ($x['id'] ?? '') === "$toon-way"))[0] ?? null;
    check(($route['name'] ?? '') === 'Way' && ($route['waypoints'][1]['name'] ?? '') === 'Bridge', 'route + free waypoint names resolved', $route);
    check(($r['body']['view']['center'] ?? null) === ['x' => 200, 'y' => 300] && ($r['body']['view']['zoom'] ?? null) === 1, 'view = the centre of the toon\'s places (no start view)', $r['body']['view'] ?? null);
    check(($r['headers']['access-control-allow-origin'] ?? '') === '*' && ($r['headers']['cache-control'] ?? '') === 'public, max-age=60', '/library/map is public and cached 60 s', $r['headers']);
    $etag = $r['headers']['etag'] ?? '';
    check(call('GET', "$api/library/map?locale=en&toon=$toon", null, null, ["If-None-Match: $etag"])['status'] === 304, '/library/map ETag → 304');
    check(call('GET', "$api/library/map?toon=x$toon")['status'] === 404, 'unknown / unlisted toon → 404');
    check(call('GET', "$api/library/map?toon=Bad*Id")['status'] === 400 && call('GET', "$api/library/map?locale=deutsch")['status'] === 400, 'invalid toon / locale → 400');
    check(call('GET', "$api/library/map?draft=1")['status'] === 401, 'draft mode needs a session');
    $r = call('GET', "$lib?locale=en");
    check(array_column($r['body']['locations'] ?? [], 'id') === ["$toon-home", "$toon-tower"] && ($r['body']['locations'][0]['chapterNumber'] ?? '') === '1'
        && ($r['body']['routes'][0]['locations'] ?? null) === ["$toon-home", "$toon-tower"], 'toon bundle: the old locations / routes shape, computed from the world map', $r['body']['locations'] ?? null);
    // a publish shows up at once (the cache is keyed by version ids)
    $world['locations'][0]['x'] = 111;
    store('map', $world);
    $r = call('GET', "$api/library/map?locale=de&toon=$toon");
    check(($r['body']['locations'][0]['x'] ?? null) === 111 || in_array(111, array_column($r['body']['locations'] ?? [], 'x'), true), 'map publish is served at once', $r['body']['locations'] ?? null);
    if ($previousMap !== null) {
        $restore = call('POST', "$api/entities/map/publish", ['version' => $previousMap], $creatorToken);
        check($restore['status'] === 200, 'the world map published before the suite is published again', $restore['raw']);
    }

    $r = call('GET', "$lib/seasons/1?locale=de");
    check($r['status'] === 200 && $r['body']['number'] === 1 && $r['body']['season']['title'] === 'Staffel de', 'season endpoint', $r['raw']);

    echo "\nChapter\n";
    $r = call('GET', "$lib/chapters/1?locale=de");
    check($r['status'] === 200 && ($r['body']['groups'][0]['label'] ?? '') === 'scene-a' && ($r['body']['groups'][1]['label'] ?? '') === 'inline',
        'one request: the ref group is the scene file', $r['raw']);
    check(str_contains($r['raw'], '"params":{}') && str_contains($r['raw'], '"volume":0.5'), 'stored JSON kept ({} objects, numbers)', $r['raw']);
    $etag = $r['headers']['etag'] ?? '';
    check(preg_match('/^"l[0-9a-f]{24}"$/', $etag) === 1, 'ETag', $etag);
    $r2 = call('GET', "$lib/chapters/1?locale=de", null, null, ["If-None-Match: $etag"]);
    check($r2['status'] === 304 && $r2['raw'] === '', 'If-None-Match → 304 without body', $r2);
    $r = call('GET', "$lib/chapters/1?locale=en");
    check($r['status'] === 200 && ($r['headers']['content-language'] ?? '') === 'de' && ($r['headers']['x-content-fallback'] ?? '') === '1',
        'en missing → de, flagged (Content-Language + X-Content-Fallback)', $r['headers']);
    $r = call('GET', "$lib/chapters/2");
    check($r['status'] === 403 && $r['body']['error'] === 'not_public', 'draft chapter → 403 not_public', $r);
    $r = call('GET', "$lib/chapters/3");
    check($r['status'] === 403 && $r['body']['error'] === 'coming_soon' && ($r['body']['chapter']['release'] ?? '') === '2027-01-01' && !isset($r['body']['chapter']['groups']),
        'coming-soon → 403 coming_soon + summary (no content)', $r['raw']);
    check(call('GET', "$lib/chapters/9")['status'] === 403, 'chapter not in the season → 403');
    check(call('GET', "$lib/chapters/1?locale=xyz")['body']['error'] === 'invalid_locale', 'invalid locale → 400');

    echo "\nPublish → visible at once (cache keyed by version ids)\n";
    store('scene:1:scene-a:de', ['label' => 'scene-a v2', 'events' => []]);
    $r = call('GET', "$lib/chapters/1?locale=de", null, null, ["If-None-Match: $etag"]);
    check($r['status'] === 200 && ($r['body']['groups'][0]['label'] ?? '') === 'scene-a v2' && ($r['headers']['etag'] ?? '') !== $etag,
        'new scene version: new ETag, new content', [$r['status'], $r['headers']['etag'] ?? null]);
    store('scene:1:scene-a:de', ['label' => 'scene-a draft'], false);
    $r = call('GET', "$lib/chapters/1?locale=de");
    check(($r['body']['groups'][0]['label'] ?? '') === 'scene-a v2', 'an unpublished draft is not public');

    echo "\nDraft mode (creator preview)\n";
    $r = call('GET', "$lib/chapters/1?draft=1");
    check($r['status'] === 401, '?draft=1 without session → 401', $r);
    $r = call('GET', "$lib/chapters/1?draft=1", null, $memberToken);
    check($r['status'] === 403 && $r['body']['error'] === 'forbidden', '?draft=1 as member → 403', $r);
    $r = call('GET', "$lib/chapters/1?draft=1", null, $creatorToken);
    check($r['status'] === 200 && ($r['body']['groups'][0]['label'] ?? '') === 'scene-a draft' && ($r['headers']['cache-control'] ?? '') === 'private, no-store',
        'creator: newest versions, not cacheable', [$r['status'], $r['headers']['cache-control'] ?? null]);
    $r = call('GET', "$lib/chapters/2?draft=1", null, $creatorToken);
    check($r['status'] === 200 && $r['body']['title'] === 'Zwei de', 'creator: draft chapters readable', $r['raw']);

    echo "\nCast, credits, events\n";
    $r = call('GET', "$lib/cast");
    check($r['status'] === 200 && array_key_exists('registry', $r['body']) && $r['body']['cast'] === null, 'cast without a cast → cast null', $r['raw']);
    check(call('GET', "$lib/credits")['status'] === 404, 'credits before any → 404');
    store('credits', ['toon' => $toon, 'sections' => []]);
    $r = call('GET', "$lib/credits");
    check($r['status'] === 200 && $r['body']['credits']['toon'] === $toon && array_key_exists('people', $r['body']), 'credits answer', $r['raw']);
    $r = call('GET', "$api/events/");
    check(in_array($r['status'], [200, 404], true) && ($r['status'] === 404 ? $r['body']['error'] === 'not_found' : is_array($r['body'])), 'events: the stored list (or 404 before the seed)', $r['status']);

    echo "\nToon visibility\n";
    store('toon', ['id' => $toon, 'status' => 'draft', 'languages' => ['de', 'en']]);
    $r = call('GET', "$lib/chapters/1");
    check($r['status'] === 404, 'draft toon → every endpoint 404', $r);
    check(!in_array($toon, array_column((array) call('GET', "$api/library/toons")['body'], 'id'), true), 'draft toon not listed');
    $r = call('GET', "$api/library/toons?draft=1", null, $creatorToken);
    check(in_array($toon, array_column((array) $r['body'], 'id'), true), 'creators list draft toons', $r['status']);
} catch (Throwable $e) {
    check(false, 'exception', $e->getMessage());
} finally {
    Db::run('DELETE FROM entity_versions WHERE toon = ?', [$toon]);
    // the suite's versions of the world map (the one published before was published again above)
    Db::run("DELETE FROM entity_versions WHERE entity_key = 'map' AND author = ?", [$creator['id']]);
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
