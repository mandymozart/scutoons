<?php
/**
 * Compare step of the data migration (agents/DATA.md §4): every library answer == the same composition done from the
 * static files the viewer reads today (JSON-equal: same values, key order ignored).
 *
 *   php tests/entities-compare.php <api base> [--static=<viewer/static/api>] [--seed=<shared/content/seed>] [--report]
 *
 * tests/run-contract.php runs it against a server on a FRESH database seeded with `import-content` from exactly these
 * folders, with CONTENT_SHOW_DRAFTS=1 (draft chapters and the sandbox are compared too). Against production (store
 * edited by creators since the seed) use --report: differences are listed, the exit code stays 0.
 */
if (PHP_SAPI !== 'cli') {
    http_response_code(404);
    exit;
}

$args = array_slice($argv, 1);
$base = rtrim((string) ($args[0] ?? 'http://localhost:9000'), '/') . '/api';
$static = __DIR__ . '/../../shared/content/seed';
$seed = __DIR__ . '/../../shared/content/seed';
$reportOnly = false;
foreach ($args as $arg) {
    if (str_starts_with($arg, '--static=')) $static = substr($arg, 9);
    elseif (str_starts_with($arg, '--seed=')) $seed = substr($arg, 7);
    elseif ($arg === '--report') $reportOnly = true;
}
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
    echo "  FAIL  $label" . ($detail !== null ? "\n        " . substr(is_string($detail) ? $detail : json_encode($detail, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE), 0, 800) : '') . "\n";
}

/** @return array{0: int, 1: mixed, 2: array<string, string>} */
function get(string $url): array
{
    $headers = [];
    $ch = curl_init($url);
    curl_setopt_array($ch, [CURLOPT_RETURNTRANSFER => true, CURLOPT_TIMEOUT => 20, CURLOPT_HEADERFUNCTION => function ($ch, $line) use (&$headers) {
        if (str_contains($line, ':')) {
            [$name, $value] = explode(':', $line, 2);
            $headers[strtolower(trim($name))] = trim($value);
        }
        return strlen($line);
    }]);
    $raw = (string) curl_exec($ch);
    $status = (int) curl_getinfo($ch, CURLINFO_RESPONSE_CODE);
    curl_close($ch);
    return [$status, json_decode($raw, true), $headers];
}

/** A static JSON file decoded as arrays, null when missing */
function file_json(string $path): mixed
{
    return is_file($path) ? json_decode((string) file_get_contents($path), true) : null;
}

/** Key order ignored, list order kept */
function canonical(mixed $value): mixed
{
    if (!is_array($value)) return $value;
    $value = array_map('canonical', $value);
    if (!array_is_list($value)) ksort($value);
    return $value;
}

function same(mixed $a, mixed $b): bool
{
    return canonical($a) === canonical($b);
}

/** The first difference of two values (for the report) */
function diff(mixed $expected, mixed $actual, string $path = '$'): ?string
{
    if (is_array($expected) && is_array($actual)) {
        foreach (array_unique(array_merge(array_keys($expected), array_keys($actual))) as $key) {
            if (!array_key_exists($key, $actual)) return "$path.$key missing in the API answer";
            if (!array_key_exists($key, $expected)) return "$path.$key only in the API answer";
            $d = diff($expected[$key], $actual[$key], "$path.$key");
            if ($d !== null) return $d;
        }
        return null;
    }
    return $expected === $actual ? null : "$path: expected " . json_encode($expected, JSON_UNESCAPED_UNICODE) . ', got ' . json_encode($actual, JSON_UNESCAPED_UNICODE);
}

function compare(string $label, mixed $expected, mixed $actual): void
{
    check(same($expected, $actual), $label, same($expected, $actual) ? null : diff(canonical($expected), canonical($actual)));
}

// the viewer's rules (isItemPublic / isItemListed with drafts – the compare server runs with CONTENT_SHOW_DRAFTS=1)
$isPublic = fn(mixed $s) => in_array(is_string($s) && $s !== '' ? $s : 'published', ['published', 'draft'], true);
$isListed = fn(mixed $s) => $isPublic($s) || $s === 'coming-soon';

/** The static file of a toon in a locale, else in the source language (the API's per-document fallback) */
$pick = function (string $toon, string $locale, string $path, string $source) use ($static): mixed {
    foreach (array_unique([$locale, $source]) as $l) {
        $doc = file_json("$static/$toon/$l/$path");
        if ($doc !== null) return $doc;
    }
    return null;
};

$summary = function (?array $chapter, array $item): array {
    $s = $chapter ?? ['title' => '', 'thumbnail' => '', 'release' => ''];
    unset($s['groups'], $s['audio'], $s['status']);
    return array_merge($s, ['type' => $item['type'] ?? 'chapter', 'number' => $item['number'], 'status' => $item['status'] ?? null]);
};

echo "API $base · static $static\n\n";

echo "Toons list\n";
$index = file_json("$static/toons.json") ?? [];
[$status, $toons] = get("$base/library/toons");
// the language-neutral toon settings live in the index entry (owner 2026-10-01, @scutoons/content TOON_NEUTRAL_FIELDS):
// the list keeps only the `tile` of them (+ `announced` for an unpublished toon with `announce: true`); the bundle merges
// them into the locale's settings and its `entry` is the index entry without them
$neutral = ['authors', 'chapters', 'thumbnail', 'map', 'announce', 'tile', 'cover'];
$withoutNeutral = function (array $t) use ($neutral): array {
    foreach ($neutral as $field) unset($t[$field]);
    return $t;
};
$listed = array_map(function (array $t) use ($withoutNeutral): array {
    $entry = $withoutNeutral($t);
    if (($t['status'] ?? null) !== 'published' && ($t['announce'] ?? null) === true) $entry['announced'] = true;
    if (is_array($t['tile'] ?? null)) $entry['tile'] = $t['tile'];
    return $entry;
}, array_values(array_filter($index, fn($t) => in_array($t['status'] ?? null, ['published', 'draft'], true))));
compare('library/toons == toons.json (published + draft entries, same order, + announced, tile)', $listed, $toons);

echo "\nEvents\n";
[$status, $events] = get("$base/events");
compare('events == events.json', file_json("$static/events.json"), $events);

/**
 * The world map (agents/MAP.md M0): `scutoons/_/map.json` replaces catpope's per-locale `locations.json` / `routes.json`
 * (kept in the seed for one release). The bundle's old fields are computed from it for older viewer builds – this proves
 * every old value survived the migration:
 *   - every old location is there with the same id, label (per locale), coordinates, toonId, type and image; only
 *     `chapterNumber` changed (owner 2026-10-01: the place's card links the chapter that introduces it, the old files
 *     said "1" everywhere) and must equal the map's `chapter`;
 *   - the other locations of the toon are exactly the map's new places (the screenplay's places, owner 2026-10-01);
 *   - every old route is there with the same id, title (`title ?? titel` – the en file's typo) and the same location ids
 *     in the same order (incl. `garden`, which is now a place);
 *   - `toon.map` equals the pre-migration values (literal below), from the world's size/terrain + the toon's start view.
 */
$world = file_json("$seed/scutoons/_/map.json");
$OLD_TOON_MAPS = [
    'catpope' => ['layers' => ['terrain' => 'map/map-layer-terrain.svg'], 'width' => 2048, 'height' => 1046, 'center' => ['x' => 1320, 'y' => 440]],
    'sandbox' => ['layers' => ['terrain' => '../catpope/map/map-layer-terrain.svg'], 'width' => 2048, 'height' => 1046, 'center' => ['x' => 1320, 'y' => 440]],
];
$compareOldMap = function (string $toon, string $locale, array $entry, array $oldLocations, array $oldRoutes, array $bundle) use ($world, $OLD_TOON_MAPS): void {
    $places = [];
    foreach ($world['locations'] ?? [] as $p) if (in_array($toon, $p['toons'] ?? [], true)) $places[$p['id']] = $p;
    $byId = array_column($bundle['locations'] ?? [], null, 'id');
    $ok = true;
    $why = [];
    foreach ($oldLocations as $old) {
        $new = $byId[$old['id']] ?? null;
        $chapter = $places[$old['id']]['chapter']['number'] ?? null;
        $expected = $old;
        if ($chapter !== null) $expected['chapterNumber'] = (string) $chapter;
        else unset($expected['chapterNumber']);
        if ($new === null || !same($expected, $new)) {
            $ok = false;
            $why[] = [$old['id'], $new];
        }
    }
    check($ok, "$toon/$locale every old location survived (id, label, coordinates, toonId, type, image; chapterNumber = the map's chapter)", $why ?: count($oldLocations));
    $extra = array_values(array_diff(array_keys($byId), array_column($oldLocations, 'id')));
    $added = array_values(array_diff(array_keys($places), array_column($oldLocations, 'id')));
    check($extra === $added, "$toon/$locale the other locations are the map's new places", ['answer' => $extra, 'map' => $added]);
    $routes = array_column($bundle['routes'] ?? [], null, 'id');
    $ok = true;
    foreach ($oldRoutes as $old) {
        $new = $routes[$old['id']] ?? null;
        $ok = $ok && $new !== null && $new['title'] === ($old['title'] ?? $old['titel'] ?? null) && $new['locations'] === $old['locations'];
    }
    check($ok && count($routes) === count($oldRoutes), "$toon/$locale every old route survived (id, title, the same place ids in order)", $bundle['routes'] ?? null);
    if (isset($OLD_TOON_MAPS[$toon])) compare("$toon/$locale toon.map == the pre-migration settings", $OLD_TOON_MAPS[$toon], $bundle['toon']['map'] ?? null);
    else check(!isset($bundle['toon']['map']) && !isset($entry['map']), "$toon/$locale no toon.map (no start view)", $bundle['toon']['map'] ?? null);
};

if ($world !== null) {
    echo "\nWorld map\n";
    [$status, $answer] = get("$base/library/map?locale=de");
    $listed = array_column(array_filter($index, fn($t) => in_array($t['status'] ?? null, ['published', 'draft'], true)), 'id');
    $readable = array_values(array_filter($world['locations'] ?? [], fn($p) => array_intersect($p['toons'] ?? [], $listed) !== []));
    check($status === 200 && array_column($answer['locations'] ?? [], 'id') === array_column($readable, 'id'), 'library/map lists the places of the readable toons, in the map\'s order', [$status, array_column($answer['locations'] ?? [], 'id')]);
    check(array_column($answer['locations'] ?? [], 'name') === array_map(fn($p) => $p['name']['de'] ?? '', $readable), 'library/map names resolved to de', array_column($answer['locations'] ?? [], 'name'));
}

foreach ($index as $entry) {
    $toon = $entry['id'];
    if (glob("$static/$toon/*/toon.json") === []) continue; // no toon settings = not readable (404), e.g. toons with a cast only
    $source = $entry['sourceLanguage'] ?? 'de';
    $languages = array_values(array_unique([$source, ...($entry['languages'] ?? [])]));
    echo "\n$toon\n";
    foreach ($languages as $locale) {
        if (!is_dir("$static/$toon/$locale")) continue;
        // the toon layout: toon.json, every season with its listed chapters (summaries), map
        $seasons = [];
        foreach (glob("$static/$toon/*/season*.json") ?: [] as $file) $seasons[(int) preg_replace('/\D/', '', basename($file))] = true;
        ksort($seasons);
        $overviews = [];
        foreach (array_keys($seasons) as $n) {
            $season = $pick($toon, $locale, "season$n.json", $source);
            $chapters = [];
            foreach ($season['sequence'] ?? [] as $item) {
                if (!$isListed($item['status'] ?? null)) continue;
                $chapters[] = $summary($pick($toon, $locale, ($item['type'] ?? 'chapter') . $item['number'] . '.json', $source), $item);
            }
            $overviews[] = ['number' => $n, 'season' => $season, 'chapters' => $chapters];
        }
        $settings = $pick($toon, $locale, 'toon.json', $source);
        foreach ($neutral as $field) if (is_array($settings) && array_key_exists($field, $entry)) $settings[$field] = $entry[$field];
        $oldLocations = $pick($toon, $locale, 'locations.json', $source) ?? [];
        $oldRoutes = $pick($toon, $locale, 'routes.json', $source) ?? [];
        $expected = [
            'entry' => $withoutNeutral($entry),
            'toon' => $settings,
            'seasons' => $overviews,
            'locations' => $oldLocations,
            'routes' => $oldRoutes,
        ];
        [$status, $bundle] = get("$base/library/toons/$toon?locale=$locale");
        if ($world !== null) {
            // the world map (agents/MAP.md §2.6): the old answer is computed from it – compared on its own below
            unset($expected['locations'], $expected['routes'], $expected['toon']['map']);
            $actual = $bundle;
            unset($actual['locations'], $actual['routes'], $actual['toon']['map']);
            compare("$toon/$locale toon bundle (without the map)", $expected, $actual);
            $compareOldMap($toon, $locale, $entry, $oldLocations, $oldRoutes, $bundle);
        } else {
            compare("$toon/$locale toon bundle", $expected, $bundle);
        }

        // every chapter of the seasons: readable ones == chapter file with its ref groups filled in; coming-soon → 403
        foreach ($overviews as $overview) {
            foreach ($overview['season']['sequence'] ?? [] as $item) {
                $n = (int) $item['number'];
                [$status, $chapter] = get("$base/library/toons/$toon/chapters/$n?locale=$locale");
                if (($item['status'] ?? null) === 'coming-soon') {
                    check($status === 403 && ($chapter['error'] ?? '') === 'coming_soon', "$toon/$locale chapter $n coming soon → 403", [$status, $chapter['error'] ?? null]);
                    continue;
                }
                $file = $pick($toon, $locale, "chapter$n.json", $source);
                if ($file === null) {
                    check($status === 404, "$toon/$locale chapter $n (no file) → 404", $status);
                    continue;
                }
                $file['groups'] = array_map(function (array $group) use ($pick, $toon, $locale, $source) {
                    $src = $group['ref']['src'] ?? null;
                    return is_string($src) ? ($pick($toon, $locale, $src, $source) ?? $group) : $group;
                }, $file['groups'] ?? []);
                compare("$toon/$locale chapter $n (scenes inlined)", $file, $chapter);
            }
        }
    }

    // cast + credits: the stored documents (static `_` folder or the seed), the registry / people reduced to what is used
    $cast = file_json("$static/$toon/_/cast.json") ?? file_json("$seed/$toon/_/cast.json");
    $registry = file_json("$static/scutoons/_/characters.json") ?? file_json("$seed/scutoons/_/characters.json");
    if ($cast !== null) {
        [$status, $answer] = get("$base/library/toons/$toon/cast");
        compare("$toon cast == cast.json", $cast, $answer['cast'] ?? null);
        $ids = [];
        foreach ($cast['seasons'] ?? [] as $entries) foreach ($entries as $e) $ids[$e['character']] = true;
        $expected = array_values(array_filter($registry['characters'] ?? [], fn($c) => isset($ids[$c['id']])));
        compare("$toon cast registry == the registry's cast characters", $expected, $answer['registry']['characters'] ?? null);
    }
    $credits = file_json("$static/$toon/_/credits.json") ?? file_json("$seed/$toon/_/credits.json");
    if ($credits !== null) {
        [$status, $answer] = get("$base/library/toons/$toon/credits");
        compare("$toon credits == credits.json", $credits, $answer['credits'] ?? null);
        compare("$toon roles == roles.json", file_json("$seed/scutoons/_/roles.json"), $answer['roles'] ?? null);
        $people = [];
        foreach (file_json("$seed/scutoons/_/people.json")['people'] ?? [] as $p) $people[$p['id']] = $p;
        $ok = true;
        foreach ($answer['people']['people'] ?? [] as $p) $ok = $ok && isset($people[$p['id']]) && same($people[$p['id']], $p);
        preg_match_all('/"person"\s*:\s*"([^"]+)"/', (string) json_encode($credits), $m);
        $missing = array_diff(array_unique($m[1]), array_column($answer['people']['people'] ?? [], 'id'));
        check($ok && $missing === [], "$toon credits people: every referenced person, as stored", ['different' => !$ok, 'missing' => array_values($missing)]);
    }
}

echo "\n" . ($checks - $failures) . "/$checks checks passed" . ($reportOnly && $failures ? " ($failures differences reported)" : '') . "\n";
exit($failures === 0 || $reportOnly ? 0 : 1);
