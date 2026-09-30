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
compare('library/toons == toons.json (published + draft entries, same order)', array_values(array_filter($index, fn($t) => in_array($t['status'] ?? null, ['published', 'draft'], true))), $toons);

echo "\nEvents\n";
[$status, $events] = get("$base/events");
compare('events == events.json', file_json("$static/events.json"), $events);

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
        $expected = [
            'entry' => $entry,
            'toon' => $pick($toon, $locale, 'toon.json', $source),
            'seasons' => $overviews,
            'locations' => $pick($toon, $locale, 'locations.json', $source) ?? [],
            'routes' => $pick($toon, $locale, 'routes.json', $source) ?? [],
        ];
        [$status, $bundle] = get("$base/library/toons/$toon?locale=$locale");
        compare("$toon/$locale toon bundle", $expected, $bundle);

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
