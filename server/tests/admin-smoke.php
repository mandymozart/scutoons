<?php
/**
 * Admin API (/api/admin/*, role admin – docs/API.md "Admin API") end-to-end against a running local server.
 *
 *   composer serve
 *   php tests/admin-smoke.php [http://localhost:9000]
 *
 * Creates throw-away users directly in the local DB (an admin and a member, with sessions), one temporary code and one
 * temporary batch (+ its files) and removes them at the end. Run it against a local copy only. Exit 0 = all passed.
 */
if (PHP_SAPI !== 'cli') {
    http_response_code(404);
    exit;
}

require_once __DIR__ . '/support/legacy.php';

use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Storage\FileStore;

$base = rtrim($argv[1] ?? 'http://localhost:9000', '/');
// The QR landing page is the qr app (in front of this API, it logs the scans through /api/code/?scan=1)
$qrBase = rtrim($argv[2] ?? 'http://localhost:9001', '/');
$api = "$base/api/admin";
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
        CURLOPT_TIMEOUT => 30,
        CURLOPT_HEADERFUNCTION => function ($ch, $line) use (&$responseHeaders) {
            if (str_contains($line, ':')) {
                [$name, $value] = explode(':', $line, 2);
                $responseHeaders[strtolower(trim($name))] = trim($value);
            }
            return strlen($line);
        },
    ]);
    if ($body !== null) curl_setopt($ch, CURLOPT_POSTFIELDS, is_string($body) ? $body : json_encode($body, JSON_UNESCAPED_SLASHES));
    $raw = (string) curl_exec($ch);
    $status = (int) curl_getinfo($ch, CURLINFO_RESPONSE_CODE);
    curl_close($ch);
    return ['status' => $status, 'body' => json_decode($raw, true) ?? $raw, 'raw' => $raw, 'headers' => $responseHeaders];
}

$run = gmdate('YmdHis') . bin2hex(random_bytes(2));
[$admin, $member] = Db::transaction(fn() => [
    Users::create("admin-smoke-$run-admin@example.com", 'en'),
    Users::create("admin-smoke-$run-member@example.com", 'en'),
]);
Roles::grant($admin['id'], Roles::ADMIN, 'admin-smoke');
$adminToken = Auth::createSession($admin['id']);
$memberToken = Auth::createSession($member['id']);
$files = new FileStore(TestContainer::get(Settings::class)->path('FILES_PATH'));
$code = null;
$batch = null;
echo "Admin API $api · admin {$admin['email']}\n\n";

try {
    echo "Access\n";
    $r = call('GET', "$api/dashboard");
    check($r['status'] === 401 && $r['body']['error'] === 'unauthorized', 'no session → 401 unauthorized', $r);
    $r = call('GET', "$api/codes", null, $memberToken);
    check($r['status'] === 403 && $r['body']['error'] === 'forbidden' && $r['body']['role'] === 'admin', 'member → 403 forbidden { role: admin }', $r);
    $r = call('GET', "$api/nope", null, $adminToken);
    check($r['status'] === 404 && $r['body']['error'] === 'not_found', 'unknown admin path → 404 not_found', $r);
    $r = call('OPTIONS', "$api/users/x/roles/creator", null, null, ['Origin: http://localhost:11000', 'Access-Control-Request-Method: DELETE']);
    check($r['status'] === 204 && ($r['headers']['access-control-allow-origin'] ?? '') === 'http://localhost:11000'
        && str_contains($r['headers']['access-control-allow-methods'] ?? '', 'DELETE'), 'CORS preflight from the admin app (DELETE allowed)', $r['headers']);

    echo "\nDashboard\n";
    $r = call('GET', "$api/dashboard?days=7", null, $adminToken);
    $d = $r['body'];
    check($r['status'] === 200 && $d['days'] === 7 && count($d['daily']) === 7 && isset($d['totals']['codes'], $d['period']['scan'], $d['editions'][0]['claimed'])
        && array_keys($d['daily'][0]) === ['day', 'scan', 'access', 'claim', 'signup', 'login', 'claim_refused', 'revoke', 'unrevoke'], 'KPIs: totals, period, editions, 7 daily rows', $d);
    $vinyl = array_values(array_filter($d['editions'], fn($e) => $e['id'] === '3_catpope_vinyl'))[0] ?? null;
    check($vinyl && $vinyl['edition_count'] === 300 && $vinyl['codes'] >= 300 && $vinyl['claimed'] >= 1, 'codes claimed / total per edition', $vinyl);
    $r = call('GET', "$api/dashboard?days=0", null, $adminToken);
    check($r['status'] === 400 && $r['body']['error'] === 'invalid_days', 'days out of range → 400', $r);
    $r = call('GET', "$api/dashboard?edition_id=2_catpope_beetles", null, $adminToken);
    check($r['status'] === 200 && count($r['body']['editions']) === 1 && $r['body']['editions'][0]['id'] === '2_catpope_beetles', 'dashboard filtered by edition', $r['body']['editions'] ?? $r);

    echo "\nCodes\n";
    $r = call('GET', "$api/codes?page_size=5&sort=-edition_number&edition_id=3_catpope_vinyl", null, $adminToken);
    $p = $r['body']['pagination'] ?? [];
    check($r['status'] === 200 && count($r['body']['items']) === 5 && $p['pageSize'] === 5 && $p['page'] === 1 && $p['total'] >= 300
        && $r['body']['items'][0]['edition_number'] >= $r['body']['items'][1]['edition_number'], 'list: page_size, sort desc, edition filter, pagination', $p);
    check(!preg_match('/@|claimed_by/', $r['raw']), 'code list carries no emails / owner ids');
    $r2 = call('GET', "$api/codes?page_size=5&page=2&sort=-edition_number&edition_id=3_catpope_vinyl", null, $adminToken);
    check(($r2['body']['items'][0]['code'] ?? null) !== ($r['body']['items'][0]['code'] ?? null) && $r2['body']['pagination']['page'] === 2, 'page 2 is another page', $r2['body']['pagination'] ?? $r2);
    $r = call('GET', "$api/codes?page_size=100000", null, $adminToken);
    check($r['body']['pagination']['pageSize'] <= 500, 'page_size is clamped', $r['body']['pagination'] ?? $r);
    $r = call('GET', "$api/codes?sort=email", null, $adminToken);
    check($r['status'] === 400 && $r['body']['error'] === 'invalid_sort', 'unknown sort field → 400 invalid_sort', $r);
    $r = call('GET', "$api/codes?status=claimed&page_size=500", null, $adminToken);
    check($r['status'] === 200 && $r['body']['items'] && !array_filter($r['body']['items'], fn($c) => $c['claimed'] !== true), 'status=claimed filter', $r['body']['pagination'] ?? $r);
    $r = call('GET', "$api/codes?status=bogus", null, $adminToken);
    check($r['status'] === 400 && $r['body']['error'] === 'invalid_status', 'unknown status → 400', $r);

    $r = call('POST', "$api/codes", ['redirect_to' => 'https://scutoons.com/catpope', 'type' => 'promo'], $adminToken);
    $code = $r['body']['code']['code'] ?? null;
    check($r['status'] === 201 && is_string($code) && strlen($code) === 8 && $r['body']['code']['type'] === 'promo', 'POST /codes creates a code (nanoid 8)', $r);
    $r = call('POST', "$api/codes", ['redirect_to' => 'https://scutoons.com/catpope', 'type' => 'promo', 'code' => $code], $adminToken);
    check($r['status'] === 409 && $r['body']['error'] === 'code_exists', 'same code again → 409 code_exists', $r);
    $r = call('POST', "$api/codes", ['redirect_to' => 'not a url', 'type' => 'promo'], $adminToken);
    check($r['status'] === 400 && $r['body']['error'] === 'invalid_redirect_to', 'invalid redirect_to → 400', $r);
    $r = call('GET', "$api/codes?q=" . rawurlencode((string) $code), null, $adminToken);
    check(($r['body']['pagination']['total'] ?? 0) === 1 && $r['body']['items'][0]['code'] === $code, 'q finds the code', $r['body']);

    call('GET', "$qrBase/?c=$code");
    $r = call('GET', "$api/codes/$code", null, $adminToken);
    check($r['status'] === 200 && $r['body']['code']['code'] === $code && $r['body']['code']['claimed'] === false && $r['body']['stats']['scans'] === 1
        && $r['body']['signed'] === false && !array_key_exists('email', $r['body']['code']), 'detail: row, stats (1 scan), no email', $r['body']);
    $r = call('GET', "$api/codes/zzzzzzzz", null, $adminToken);
    check($r['status'] === 404, 'unknown code → 404', $r);
    $r = call('POST', "$api/codes/$code/revoke", ['reason' => 'admin smoke'], $adminToken);
    check($r['status'] === 200 && $r['body']['revoked_reason'] === 'admin smoke' && $r['body']['revoked_at'] !== null, 'revoke with reason', $r);
    $r = call('GET', "$base/api/access/?c=$code");
    check($r['status'] === 410 && $r['body']['error'] === 'revoked', 'revoked code → /access/ 410', $r);
    $r = call('POST', "$api/codes/$code/unrevoke", null, $adminToken);
    check($r['status'] === 200 && $r['body']['revoked_at'] === null, 'unrevoke', $r);
    $r = call('GET', "$api/codes/$code/events", null, $adminToken);
    check($r['status'] === 200 && array_column($r['body']['items'], 'event') === ['unrevoke', 'access', 'revoke', 'scan'] || array_column($r['body']['items'], 'event') === ['unrevoke', 'revoke', 'scan'],
        'timeline newest first (unrevoke, revoke, scan)', array_column($r['body']['items'] ?? [], 'event'));

    $sig = ['signature' => str_repeat('0f', 40), 'publicKey' => 'ab', 'algorithm' => 'ML-DSA-65', 'signedData' => json_encode(['code' => $code]), 'timestamp' => gmdate('c')];
    $r = call('PUT', "$api/codes/$code/signature", ['signature' => ['algorithm' => 'x']], $adminToken);
    check($r['status'] === 400 && $r['body']['error'] === 'invalid_signature', 'signature without fields → 400', $r);
    $r = call('PUT', "$api/codes/$code/signature", ['signature' => $sig], $adminToken);
    check($r['status'] === 200, 'store a signature', $r);
    $r = call('GET', "$api/codes/$code/signature", null, $adminToken);
    check($r['status'] === 200 && $r['body']['signature']['signature'] === $sig['signature'] && $r['body']['qr_data']['code'] === $code
        && str_starts_with($r['body']['fingerprint'], '0F0F:0F0F'), 'read the signature (fingerprint, qr_data)', $r['body']);

    $ch = curl_init("$api/codes?format=csv&edition_id=2_catpope_beetles");
    curl_setopt_array($ch, [CURLOPT_RETURNTRANSFER => true, CURLOPT_HTTPHEADER => ["Authorization: Bearer $adminToken"]]);
    $csv = (string) curl_exec($ch);
    curl_close($ch);
    check(str_starts_with($csv, 'code,type,edition_id') && substr_count(trim($csv), "\n") === 40, 'CSV export of a filtered list (40 beetles codes)', substr($csv, 0, 120));

    echo "\nEvents\n";
    $r = call('GET', "$api/events?code=$code&page_size=2", null, $adminToken);
    check($r['status'] === 200 && count($r['body']['items']) === 2 && $r['body']['pagination']['total'] >= 3 && $r['body']['items'][0]['event'] === 'unrevoke'
        && !array_key_exists('user_id', $r['body']['items'][0]), 'events list: filter code, newest first, paged, no user ids', $r['body']);
    $r = call('GET', "$api/events?event=scan,revoke&code=$code", null, $adminToken);
    check($r['status'] === 200 && array_unique(array_column($r['body']['items'], 'event')) == ['revoke', 'scan'], 'events filter by several event types', array_column($r['body']['items'] ?? [], 'event'));
    $r = call('GET', "$api/events?event=nope", null, $adminToken);
    check($r['status'] === 400 && $r['body']['error'] === 'invalid_event', 'unknown event → 400', $r);
    $r = call('GET', "$api/events?from=2026-13", null, $adminToken);
    check($r['status'] === 400 && $r['body']['error'] === 'invalid_date', 'bad date → 400', $r);

    echo "\nBatches and files\n";
    $r = call('GET', "$api/batches?sort=-created_at", null, $adminToken);
    check($r['status'] === 200 && isset($r['body']['pagination']['total']) && ($r['body']['items'][0]['filename'] ?? '') !== '', 'batches list', $r['body']['pagination'] ?? $r);
    $r = call('POST', "$api/batches", ['edition_id' => '3_catpope_vinyl', 'count' => 2, 'start_number' => 901], $adminToken);
    check($r['status'] === 409 && $r['body']['error'] === 'batch_complete', 'new batch of a completed edition → 409 batch_complete', $r);
    $r = call('POST', "$api/batches", ['edition_id' => '3_catpope_vinyl', 'count' => 2, 'start_number' => 901, 'force' => true], $adminToken);
    $batch = $r['body']['batch'] ?? null;
    check($r['status'] === 201 && count($batch['codes'] ?? []) === 2 && $batch['codes'][0]['n'] === 901 && $batch['redirectUrl'] === 'https://scutoons.com/catpope'
        && $batch['codes'][0]['status'] === 'pending' && Codes::find($batch['codes'][0]['code']) === null, 'create a batch (force): 2 reserved codes, not in the DB yet', $batch);
    $filename = $batch['filename'] ?? 'x';
    $batchCode = $batch['codes'][0]['code'] ?? 'x';
    $r = call('POST', "$api/codes", ['code' => $batchCode, 'redirect_to' => $batch['redirectUrl'], 'type' => 'membership', 'edition_id' => '3_catpope_vinyl'], $adminToken);
    check($r['status'] === 201, 'register a batch code', $r);
    $r = call('PUT', "$api/files/svgs/SCUTOONS-QR-MEMBERSHIP-$batchCode.svg", ['doc' => '<svg xmlns="http://www.w3.org/2000/svg"/>'], $adminToken);
    check($r['status'] === 200, 'store the QR SVG', $r);
    $r = call('PUT', "$api/files/certificates/SCUTOONS-COA-$batchCode.pdf", ['doc' => 'data:application/pdf;base64,' . base64_encode('%PDF-1.4')], $adminToken);
    check($r['status'] === 200 && $files->read('certificates', "SCUTOONS-COA-$batchCode.pdf") === '%PDF-1.4', 'store the certificate (data URI decoded)', $r);
    $r = call('PUT', "$api/files/evil/x.svg", ['doc' => 'x'], $adminToken);
    check($r['status'] === 400 && $r['body']['error'] === 'invalid_type', 'unknown file type → 400', $r);
    $r = call('PUT', "$api/files/svgs/..x.svg", ['doc' => 'x'], $adminToken);
    check(in_array($r['status'], [400, 404], true), 'dot file name → 400 (or 404 from the router)', $r);
    $r = call('GET', "$api/files/svgs/SCUTOONS-QR-MEMBERSHIP-$batchCode.svg", null, $adminToken);
    check($r['status'] === 200 && str_contains($r['raw'], '<svg') && str_starts_with($r['headers']['content-type'] ?? '', 'image/svg+xml'), 'read a stored file', $r['headers']);
    $batch['codes'][0]['status'] = 'done';
    $batch['codes'][0]['completedSteps'] = ['db', 'svg', 'pdf'];
    $r = call('PUT', "$api/batches/$filename", ['manifest' => $batch], $adminToken);
    check($r['status'] === 200 && $r['body']['batch']['codes'][0]['status'] === 'done', 'store manifest progress', $r);
    $r = call('PUT', "$api/batches/$filename", ['manifest' => ['codes' => 'nope']], $adminToken);
    check($r['status'] === 422 && $r['body']['error'] === 'invalid_manifest', 'invalid manifest → 422', $r);
    $r = call('GET', "$api/batches/$filename", null, $adminToken);
    check($r['status'] === 200 && $r['body']['batch']['filename'] === $filename, 'read the manifest', $r);
    $r = call('GET', "$api/batches/BATCH-nope.json", null, $adminToken);
    check($r['status'] === 404, 'unknown batch → 404', $r);
    $r = call('GET', "$api/batches/$filename/zip", null, $adminToken);
    $tmp = tempnam(sys_get_temp_dir(), 'zip');
    file_put_contents($tmp, $r['raw']);
    $zip = new ZipArchive();
    $names = [];
    if ($zip->open($tmp) === true) {
        for ($i = 0; $i < $zip->numFiles; $i++) $names[] = $zip->getNameIndex($i);
        $zip->close();
    }
    @unlink($tmp);
    sort($names);
    check($r['status'] === 200 && $names === ["certificates/SCUTOONS-COA-$batchCode.pdf", 'manifest.json', "svgs/SCUTOONS-QR-MEMBERSHIP-$batchCode.svg"], 'ZIP: manifest + files of finished codes', $names);
    $r = call('GET', "$api/batches?q=" . rawurlencode($filename), null, $adminToken);
    check(($r['body']['pagination']['total'] ?? 0) === 1 && $r['body']['items'][0]['done'] === 1, 'batch list q + progress', $r['body']);

    echo "\nUsers and roles\n";
    $r = call('GET', "$api/roles", null, $adminToken);
    check($r['status'] === 200 && array_column($r['body']['items'], 'name') === ['member', 'admin', 'creator'], 'roles', $r);
    $r = call('GET', "$api/users?q=" . rawurlencode("admin-smoke-$run") . '&sort=email', null, $adminToken);
    check($r['status'] === 200 && $r['body']['pagination']['total'] === 2 && $r['body']['items'][0]['email'] === $admin['email'] && $r['body']['items'][0]['roles'] === ['member', 'admin'], 'users list: q, sort, roles', $r['body']);
    $r = call('GET', "$api/users?role=admin&q=" . rawurlencode("admin-smoke-$run"), null, $adminToken);
    check(($r['body']['pagination']['total'] ?? 0) === 1, 'users filter by role', $r['body']);
    $r = call('GET', "$api/users/{$member['id']}", null, $adminToken);
    check($r['status'] === 200 && $r['body']['user']['email'] === $member['email'] && $r['body']['user']['sessions'] === 1 && $r['body']['user']['codes'] === [], 'user detail', $r['body']);
    $r = call('PUT', "$api/users/{$member['id']}/roles/creator", null, $adminToken);
    check($r['status'] === 200 && $r['body']['changed'] === true && $r['body']['user']['roles'] === ['member', 'creator'] && Roles::has($member['id'], 'creator'), 'grant creator', $r);
    $r = call('PUT', "$api/users/{$member['id']}/roles/creator", null, $adminToken);
    check($r['status'] === 200 && $r['body']['changed'] === false, 'grant again → unchanged', $r);
    $r = call('DELETE', "$api/users/{$member['id']}/roles/creator", null, $adminToken);
    check($r['status'] === 200 && $r['body']['changed'] === true && !Roles::has($member['id'], 'creator'), 'revoke creator', $r);
    $r = call('PUT', "$api/users/{$member['id']}/roles/wizard", null, $adminToken);
    check($r['status'] === 400 && $r['body']['error'] === 'unknown_role', 'unknown role → 400', $r);
    $r = call('PUT', "$api/users/nope/roles/creator", null, $adminToken);
    check($r['status'] === 404, 'unknown user → 404', $r);

    echo "\nCatalog and keys\n";
    $r = call('GET', "$api/catalog", null, $adminToken);
    check($r['status'] === 200 && count($r['body']['catalog']['items']) === 9 && isset($r['body']['permissions']['types']['membership']) && count($r['body']['toons']) >= 1, 'catalog + editions + toons + permissions map', array_keys((array) $r['body']));
    $r = call('GET', "$api/keys", null, $adminToken);
    check(($r['status'] === 200 && is_string($r['body']['privateKey'] ?? null) && is_string($r['body']['publicKey'] ?? null)) || ($r['status'] === 404 && $r['body']['error'] === 'keys_missing'),
        'keys for client-side signing (admin only)', [$r['status'], array_keys((array) $r['body'])]);
    $r = call('GET', "$api/keys", null, $memberToken);
    check($r['status'] === 403, 'keys for a member → 403', $r['status']);

    echo "\nContent store overview\n";
    $contentToon = 'admin-smoke-' . strtolower($run);
    $now = gmdate('Y-m-d H:i:s');
    Db::run("INSERT INTO entity_versions (entity_key, kind, toon, locale, json, author, created_at, status, message) VALUES (?, 'chapter', ?, 'de', '{}', NULL, ?, 'published', 'smoke')", ["chapter:$contentToon:1:de", $contentToon, $now]);
    Db::run("INSERT INTO entity_versions (entity_key, kind, toon, locale, json, author, created_at, status, message) VALUES (?, 'chapter', ?, 'de', '{\"a\":1}', NULL, ?, 'draft', 'smoke')", ["chapter:$contentToon:1:de", $contentToon, $now]);
    $r = call('GET', "$api/content", null, $adminToken);
    $mine = array_values(array_filter((array) ($r['body']['entities'] ?? []), fn($f) => ($f['toon'] ?? null) === $contentToon));
    check($r['status'] === 200 && in_array('catpope', array_column((array) ($r['body']['toons'] ?? []), 'id'), true) && in_array($contentToon, array_column((array) ($r['body']['toons'] ?? []), 'id'), true),
        'content: toons from config + store', array_keys((array) $r['body']));
    check(count($mine) === 1 && $mine[0]['key'] === "chapter:$contentToon:1:de" && $mine[0]['kind'] === 'chapter' && $mine[0]['locale'] === 'de' && $mine[0]['versions'] === 2 && $mine[0]['latest_status'] === 'draft' && is_int($mine[0]['published']) && $mine[0]['published'] < $mine[0]['latest'] && $mine[0]['published_at'] === $now,
        'content: newest draft ahead of the published version', $mine);
    $r = call('GET', "$api/content?toon=$contentToon", null, $adminToken);
    check($r['status'] === 200 && count($r['body']['entities']) === 1 && count($r['body']['toons']) === 1, 'content ?toon= filter', $r['body']);
    $r = call('GET', "$api/content?toon=" . urlencode('Bad Toon!'), null, $adminToken);
    check($r['status'] === 400 && $r['body']['error'] === 'invalid_toon', 'content: invalid toon → 400', $r);
    $r = call('GET', "$api/content", null, $memberToken);
    check($r['status'] === 403, 'content for a member → 403', $r['status']);

    // Runtime settings + the scan limit, end to end through the qr app (it forwards the client IP, 127.0.0.1 here)
    echo "\nSettings\n";
    $r = call('GET', "$api/settings");
    check($r['status'] === 401, 'settings without a session → 401', $r['status']);
    $r = call('PUT', "$api/settings", ['scan_limit_per_hour' => 3], $memberToken);
    check($r['status'] === 403 && $r['body']['error'] === 'forbidden', 'settings update by a member → 403', $r);
    $r = call('GET', "$api/settings", null, $adminToken);
    $settingsBefore = array_column((array) ($r['body']['items'] ?? []), null, 'key');
    check($r['status'] === 200 && isset($settingsBefore['scan_limit_per_hour']['value'], $settingsBefore['lookup_limit_per_hour']['default'])
        && array_key_exists('updated_by', $settingsBefore['scan_limit_per_hour']), 'GET settings: scan + lookup limit with value, default, last change', $r['body']);
    $r = call('PUT', "$api/settings", ['scan_limit_per_hour' => -1], $adminToken);
    check($r['status'] === 400 && $r['body']['error'] === 'invalid_setting', 'negative limit → 400 invalid_setting', $r);
    $r = call('PUT', "$api/settings", ['scan_limit_per_hour' => 3], $adminToken);
    $scanSetting = array_column((array) ($r['body']['items'] ?? []), null, 'key')['scan_limit_per_hour'] ?? [];
    check($r['status'] === 200 && ($scanSetting['value'] ?? null) === 3 && ($scanSetting['updated_by'] ?? null) === "admin:{$admin['email']}",
        'PUT scan_limit_per_hour = 3 (updated_by = the admin)', $r['body']);

    $secrets = TestContainer::get(Scutoons\Domain\Secrets::class);
    $scanBuckets = array_map(fn(string $ip) => 'qrscan:' . $secrets->hmac($ip), ['127.0.0.1', '203.0.113.77']);
    $clearScanBuckets = fn() => Db::run('DELETE FROM rate_limits WHERE bucket IN (?, ?)', $scanBuckets);
    $clearScanBuckets(); // earlier suites scanned from 127.0.0.1 too
    $scanCode = call('POST', "$api/codes", ['redirect_to' => 'https://scutoons.com/catpope', 'type' => 'promo'], $adminToken)['body']['code']['code'] ?? null;
    $statuses = [];
    for ($i = 0; $i < 4; $i++) $statuses[] = call('GET', "$qrBase/?c=$scanCode");
    check(array_column(array_slice($statuses, 0, 3), 'status') === [200, 200, 200], 'scans 1–3 through the qr app → 200', array_column($statuses, 'status'));
    check($statuses[3]['status'] === 429 && str_contains($statuses[3]['raw'], 'Too many codes checked'), 'scan 4 → the qr app\'s 429 page', $statuses[3]['status']);
    $proxy = ['X-Scutoons-Proxy-Secret: ' . TestContainer::get(Settings::class)->get('QR_PROXY_SECRET'), 'X-Scutoons-Client-Ip: 203.0.113.77'];
    $r = call('GET', "$base/api/code/?c=$scanCode&scan=1", null, null, $proxy);
    check($r['status'] === 200, 'another forwarded client IP still scans (own bucket)', $r['status']);
    $r = call('GET', "$base/api/code/?c=$scanCode");
    check($r['status'] === 200, 'a direct lookup from the limited IP still works (lookups have their own bucket)', $r['status']);
    $r = call('PUT', "$api/settings", ['scan_limit_per_hour' => 0], $adminToken);
    $statuses = [];
    for ($i = 0; $i < 5; $i++) $statuses[] = call('GET', "$qrBase/?c=$scanCode")['status'];
    check($r['status'] === 200 && $statuses === [200, 200, 200, 200, 200], 'scan_limit_per_hour = 0 → unlimited', $statuses);

    echo "\nAvatar bodies & character unlocks (default: catpope swim + naked, AVATAR_DEFAULT_UNLOCKS)\n";
    // The unlock rules read the published character registry of the content store; a database without one gets a
    // minimal registry for the suite (removed at the end)
    $registryRow = Db::one("SELECT id FROM entity_versions WHERE entity_key = 'characters' AND status = 'published' ORDER BY id DESC LIMIT 1");
    if (!$registryRow) {
        $apose = fn(string $c, array $looks) => ['looks' => array_map(fn($l, $i) => ['id' => $l, 'default' => $i === 0, 'path' => "characters/$c/{$c}_apose-$l.webp", 'slots' => [['id' => 'torso', 'slot' => 'torso', 'x' => 1, 'y' => 1, 'size' => 1]]], $looks, array_keys($looks))];
        $json = json_encode(['characters' => [
            ['id' => 'catpope', 'name' => ['en' => 'Cat Pope'], 'color' => 'blue', 'apose' => $apose('catpope', ['swim', 'naked']), 'personas' => []],
            ['id' => 'yellowmouse', 'name' => ['en' => 'Yellow Mouse'], 'color' => 'yellow', 'apose' => $apose('yellowmouse', ['swim', 'naked']), 'personas' => []],
            ['id' => 'toptoy', 'name' => ['en' => 'Toptoy'], 'color' => 'black', 'personas' => []],
        ]], JSON_UNESCAPED_SLASHES);
        Db::run("INSERT INTO entity_versions (entity_key, kind, toon, locale, json, author, created_at, status, message) VALUES ('characters', 'characters', NULL, NULL, ?, NULL, ?, 'published', 'admin-smoke')", [$json, Db::now()]);
        $testRegistryId = (int) Db::value('SELECT MAX(id) FROM entity_versions');
    }
    $userApi = "$base/api";
    $r = call('GET', "$userApi/me/", null, $memberToken);
    $av = $r['body']['avatar'] ?? [];
    check($r['status'] === 200 && array_key_exists('body', $av) && $av['body'] === null
        && ($av['unlocked'] ?? null) === [['character' => 'catpope', 'looks' => ['swim', 'naked'], 'source' => 'default']], 'GET /me/: avatar.body null, unlocked = the default (catpope swim + naked)', $av);
    $r = call('PUT', "$userApi/avatar/", ['body' => ['character' => 'yellowmouse', 'look' => 'swim']], $memberToken);
    check($r['status'] === 422 && $r['body']['error'] === 'locked', 'PUT /avatar/ body of a locked character → 422 locked', $r);
    $r = call('PUT', "$userApi/avatar/", ['body' => ['character' => 'catpope', 'look' => 'bogus']], $memberToken);
    check($r['status'] === 422 && $r['body']['error'] === 'locked', 'unknown look → 422 locked', $r);
    $r = call('PUT', "$userApi/avatar/", ['body' => 'catpope'], $memberToken);
    check($r['status'] === 400 && $r['body']['error'] === 'invalid_body', 'body not { character, look } → 400 invalid_body', $r);
    $r = call('PUT', "$userApi/avatar/", [], $memberToken);
    check($r['status'] === 400 && $r['body']['error'] === 'invalid_slots', 'neither slots nor body → 400 invalid_slots', $r);
    $r = call('PUT', "$userApi/avatar/", ['body' => ['character' => 'catpope', 'look' => 'naked']], $memberToken);
    check($r['status'] === 200 && ($r['body']['avatar']['body'] ?? null) === ['character' => 'catpope', 'look' => 'naked'] && isset($r['body']['avatar']['slots']), 'PUT /avatar/ { body } a default body → saved', $r);
    $r = call('PUT', "$api/users/{$member['id']}/characters/yellowmouse", null, $memberToken);
    check($r['status'] === 403, 'member cannot unlock characters (admin API)', $r);
    $r = call('PUT', "$api/users/{$member['id']}/characters/nobody", null, $adminToken);
    check($r['status'] === 400 && $r['body']['error'] === 'unknown_character', 'unlock an unknown character → 400 unknown_character', $r);
    $r = call('PUT', "$api/users/{$member['id']}/characters/yellowmouse", ['looks' => ['glitter']], $adminToken);
    check($r['status'] === 400 && $r['body']['error'] === 'invalid_look', 'unlock an unknown A-pose look → 400 invalid_look', $r);
    $r = call('PUT', "$api/users/nobody/characters/yellowmouse", null, $adminToken);
    check($r['status'] === 404, 'unknown user → 404', $r);
    $r = call('PUT', "$api/users/{$member['id']}/characters/yellowmouse", null, $adminToken);
    $mouse = array_values(array_filter($r['body']['characters'] ?? [], fn($c) => $c['character'] === 'yellowmouse'))[0] ?? null;
    check($r['status'] === 200 && $r['body']['changed'] === true && $mouse && $mouse['looks'] === ['swim'] && $mouse['source'] === 'admin'
        && str_starts_with((string) $mouse['ref'], 'admin:') && $mouse['unlocked_at'] !== null, 'admin unlocks yellowmouse → its default A-pose look (swim), source admin, ref admin:<email>', $r);
    $r = call('PUT', "$api/users/{$member['id']}/characters/yellowmouse", null, $adminToken);
    check($r['status'] === 200 && $r['body']['changed'] === false, 'unlocking again → changed false', $r);
    $r = call('GET', "$api/users/{$member['id']}", null, $adminToken);
    check(array_column($r['body']['user']['characters'] ?? [], 'character') === ['catpope', 'yellowmouse'], 'admin user detail lists the unlocks (default + granted)', $r['body']['user']['characters'] ?? $r);
    $r = call('PUT', "$userApi/avatar/", ['body' => ['character' => 'yellowmouse', 'look' => 'naked']], $memberToken);
    check($r['status'] === 422 && $r['body']['error'] === 'locked', 'a look beyond the default stays locked', $r);
    $r = call('PUT', "$userApi/avatar/", ['body' => ['character' => 'yellowmouse', 'look' => 'swim']], $memberToken);
    check($r['status'] === 200 && ($r['body']['avatar']['body'] ?? null) === ['character' => 'yellowmouse', 'look' => 'swim'], 'the member wears the unlocked mouse (swim)', $r);
    $r = call('PUT', "$api/users/{$member['id']}/characters/yellowmouse", ['looks' => ['naked'], 'source' => 'item', 'ref' => 'admin-smoke'], $adminToken);
    $mouse = array_values(array_filter($r['body']['characters'] ?? [], fn($c) => $c['character'] === 'yellowmouse'))[0] ?? null;
    check($r['status'] === 200 && $r['body']['changed'] === true && $mouse && $mouse['looks'] === ['swim', 'naked'], 'unlock a further look (naked, source item)', $r);
    $r = call('PUT', "$userApi/avatar/", ['body' => ['character' => 'yellowmouse', 'look' => 'naked'], 'slots' => new stdClass()], $memberToken);
    check($r['status'] === 200 && ($r['body']['avatar']['body']['look'] ?? null) === 'naked', 'body + slots in one PUT', $r);
    $r = call('DELETE', "$api/users/{$member['id']}/characters/yellowmouse/looks/naked", null, $adminToken);
    $me = call('GET', "$userApi/me/", null, $memberToken)['body']['avatar'] ?? [];
    check($r['status'] === 200 && $r['body']['changed'] === true && $me['body'] === null, 'revoking the worn look resets avatar.body', [$r, $me]);
    $r = call('DELETE', "$api/users/{$member['id']}/characters/yellowmouse", null, $adminToken);
    $me = call('GET', "$userApi/me/", null, $memberToken)['body']['avatar'] ?? [];
    check($r['status'] === 200 && $r['body']['changed'] === true && array_column($me['unlocked'] ?? [], 'character') === ['catpope'], 'revoke the character → only the default is left', [$r, $me]);
    $r = call('DELETE', "$api/users/{$member['id']}/characters/catpope", null, $adminToken);
    check($r['status'] === 200 && $r['body']['changed'] === false && array_column($r['body']['characters'], 'character') === ['catpope'], 'the default unlock cannot be revoked per member', $r);
    $r = call('PUT', "$userApi/avatar/", ['body' => null], $memberToken);
    check($r['status'] === 200 && $r['body']['avatar']['body'] === null, 'body null → back to the default', $r);

    echo "\nSlots per avatar body (twin slot instances, agents/ITEMS.md)\n";
    // A registry version for this part (newest published, removed at the end): the current registry + a duo "toptoy"
    // whose swim body has head/torso twice (left/right) and catpope's swim with one instance per type
    $current = Db::one("SELECT json FROM entity_versions WHERE entity_key = 'characters' AND status = 'published' ORDER BY id DESC LIMIT 1");
    $registry = json_decode((string) ($current['json'] ?? '{"characters":[]}'), true);
    $inst = fn(string $id, string $slot, int $x) => ['id' => $id, 'slot' => $slot, 'x' => $x, 'y' => 100, 'size' => 50];
    $single = array_map(fn($t) => $inst($t, $t, 380), ['head', 'face', 'neck', 'torso', 'hands', 'legs', 'feet', 'back', 'accessory']);
    $twins = [$inst('head-left', 'head', 200), $inst('torso-left', 'torso', 200), $inst('head-right', 'head', 570), $inst('torso-right', 'torso', 570)];
    $registry['characters'] = array_values(array_filter($registry['characters'] ?? [], fn($c) => !in_array($c['id'] ?? null, ['catpope', 'toptoy'], true)));
    $registry['characters'][] = ['id' => 'catpope', 'name' => ['en' => 'Cat Pope'], 'color' => 'blue', 'personas' => [], 'apose' => ['looks' => [
        ['id' => 'swim', 'default' => true, 'path' => 'characters/catpope/catpope_apose-swim.webp', 'slots' => $single],
        ['id' => 'naked', 'path' => 'characters/catpope/catpope_apose-naked.webp'],
    ]]];
    $registry['characters'][] = ['id' => 'toptoy', 'name' => ['en' => 'Toptoy'], 'color' => 'black', 'personas' => [], 'apose' => ['looks' => [
        ['id' => 'swim', 'default' => true, 'path' => 'characters/toptoy/toptoy_apose-swim.webp', 'slots' => $twins],
    ]]];
    Db::run("INSERT INTO entity_versions (entity_key, kind, toon, locale, json, author, created_at, status, message) VALUES ('characters', 'characters', NULL, NULL, ?, NULL, ?, 'published', 'admin-smoke twins')", [json_encode($registry, JSON_UNESCAPED_SLASHES), Db::now()]);
    $twinRegistryId = (int) Db::value('SELECT MAX(id) FROM entity_versions');
    // two torso wearables (the same slot type) and a head-less item type check
    $wear = [];
    foreach (['hoodie' => ['dago_hoodie', 'black'], 'tee' => ['dago_heavy_t', 'white']] as $k => [$itemId, $variant]) {
        $wear[$k] = 'admin-smoke-' . $k . '-' . bin2hex(random_bytes(4));
        Db::run("INSERT INTO inventory (id, user_id, item_id, variant, qty, source, code, edition_number, acquired_at, updated_at) VALUES (?, ?, ?, ?, 1, 'story', NULL, NULL, ?, ?)", [$wear[$k], $member['id'], $itemId, $variant, Db::now(), Db::now()]);
    }
    $r = call('PUT', "$userApi/avatar/", ['slots' => ['torso' => $wear['hoodie']]], $memberToken);
    check($r['status'] === 200 && ($r['body']['avatar']['slots']['torso'] ?? null) === $wear['hoodie'], 'default body (catpope swim, one instance per type): hoodie on torso', $r);
    $r = call('PUT', "$userApi/avatar/", ['slots' => ['torso-left' => $wear['tee']]], $memberToken);
    check($r['status'] === 422 && $r['body']['error'] === 'invalid_slot' && $r['body']['slot'] === 'torso-left', 'an instance the body does not have → 422 invalid_slot', $r);
    $r = call('PUT', "$api/users/{$member['id']}/characters/toptoy", null, $adminToken);
    check($r['status'] === 200 && $r['body']['changed'] === true, 'admin unlocks the duo (toptoy)', $r);
    $r = call('PUT', "$userApi/avatar/", ['body' => ['character' => 'toptoy', 'look' => 'swim']], $memberToken);
    $slots = (array) ($r['body']['avatar']['slots'] ?? []);
    check($r['status'] === 200 && array_keys($slots) === ['head-left', 'torso-left', 'head-right', 'torso-right'] && $slots['torso-left'] === $wear['hoodie'] && $slots['torso-right'] === null,
        'switch to the duo: slots = its instances, the hoodie moves by type to the first free torso (torso-left)', $r);
    $r = call('PUT', "$userApi/avatar/", ['slots' => ['torso-right' => $wear['tee']]], $memberToken);
    $slots = (array) ($r['body']['avatar']['slots'] ?? []);
    check($r['status'] === 200 && $slots['torso-left'] === $wear['hoodie'] && $slots['torso-right'] === $wear['tee'], 'two torso items at once: hoodie left, tee right', $r);
    $r = call('PUT', "$userApi/avatar/", ['slots' => ['head-right' => $wear['tee']]], $memberToken);
    check($r['status'] === 422 && $r['body']['error'] === 'wrong_slot' && ($r['body']['slot_type'] ?? null) === 'head', 'a torso item on a head instance → 422 wrong_slot', $r);
    $r = call('PUT', "$userApi/avatar/", ['slots' => ['torso-left' => $wear['tee']]], $memberToken);
    $slots = (array) ($r['body']['avatar']['slots'] ?? []);
    check($r['status'] === 200 && $slots['torso-left'] === $wear['tee'] && $slots['torso-right'] === null && $slots['head-left'] === null, 'one entry is worn once: the tee moves left, the hoodie comes off (left instance taken)', $r);
    $r = call('PUT', "$userApi/avatar/", ['slots' => ['torso-right' => $wear['hoodie']]], $memberToken);
    $me = call('GET', "$userApi/me/", null, $memberToken)['body']['avatar'] ?? [];
    check((array) ($me['slots'] ?? []) === ['head-left' => null, 'torso-left' => $wear['tee'], 'head-right' => null, 'torso-right' => $wear['hoodie']], 'GET /me/ lists the body\'s instances with both worn', $me);
    $r = call('PUT', "$userApi/avatar/", ['body' => ['character' => 'catpope', 'look' => 'swim']], $memberToken);
    $slots = (array) ($r['body']['avatar']['slots'] ?? []);
    check($r['status'] === 200 && count($slots) === 9 && $slots['torso'] === $wear['tee'] && !in_array($wear['hoodie'], $slots, true),
        'back to a single body: the first torso keeps the one torso instance, the second comes off', $r);
    $r = call('PUT', "$userApi/avatar/", ['body' => ['character' => 'toptoy', 'look' => 'swim']], $memberToken);
    $slots = (array) ($r['body']['avatar']['slots'] ?? []);
    check($r['status'] === 200 && $slots['torso-left'] === $wear['tee'] && $slots['torso-right'] === null, 'and to the duo again: by type to torso-left', $r);
    // a legacy row (stored by type) on the duo is read by type
    Db::run('DELETE FROM avatar_slots WHERE user_id = ?', [$member['id']]);
    Db::run("INSERT INTO avatar_slots (user_id, slot, entry_id, updated_at) VALUES (?, 'torso', ?, ?)", [$member['id'], $wear['hoodie'], Db::now()]);
    $me = call('GET', "$userApi/me/", null, $memberToken)['body']['avatar'] ?? [];
    check(($me['slots']['torso-left'] ?? null) === $wear['hoodie'], 'a row stored by slot type (before the instances) shows on the first instance of that type', $me);
    $r = call('PUT', "$userApi/avatar/", ['slots' => ['torso-right' => $wear['tee']]], $memberToken);
    $rows = Db::all('SELECT slot, entry_id FROM avatar_slots WHERE user_id = ? ORDER BY slot', [$member['id']]);
    check(array_column($rows, 'entry_id', 'slot') === ['torso-left' => $wear['hoodie'], 'torso-right' => $wear['tee']], 'the next save stores it by instance id', $rows);
} finally {
    if (isset($twinRegistryId)) Db::run('DELETE FROM entity_versions WHERE id = ?', [$twinRegistryId]);
    // Back to the limits from before the suite, and no test rate-limit rows left
    if (isset($settingsBefore) && $settingsBefore) {
        call('PUT', "$api/settings", array_map(fn($s) => $s['stored'] ? $s['value'] : null, $settingsBefore), $adminToken);
    }
    if (isset($clearScanBuckets)) $clearScanBuckets();
    if (isset($scanCode) && is_string($scanCode)) {
        Db::run('DELETE FROM code_events WHERE code = ?', [$scanCode]);
        Db::run('DELETE FROM qr WHERE code = ?', [$scanCode]);
    }
    if (isset($contentToon)) Db::run('DELETE FROM entity_versions WHERE toon = ?', [$contentToon]);
    foreach (array_filter([$code, $batch['codes'][0]['code'] ?? null, $batch['codes'][1]['code'] ?? null]) as $c) {
        Db::run('DELETE FROM code_events WHERE code = ?', [$c]);
        Db::run('DELETE FROM qr WHERE code = ?', [$c]);
        foreach ([['svgs', "SCUTOONS-QR-MEMBERSHIP-$c.svg"], ['certificates', "SCUTOONS-COA-$c.pdf"], ['signatures', "SIGNATURE-$c.json"]] as [$type, $name]) {
            if ($files->exists($type, $name)) unlink($files->path($type, $name));
        }
    }
    if (isset($batch['filename'])) {
        foreach ([['batch', $batch['filename']], ['zips', str_replace('.json', '.zip', $batch['filename'])]] as [$type, $name]) {
            if ($files->exists($type, $name)) unlink($files->path($type, $name));
        }
    }
    if (isset($testRegistryId)) Db::run('DELETE FROM entity_versions WHERE id = ?', [$testRegistryId]);
    foreach ([$admin, $member] as $user) {
        foreach (['member_characters', 'member_character_looks', 'avatar_body', 'avatar_slots'] as $table) Db::run("DELETE FROM $table WHERE user_id = ?", [$user['id']]);
        Db::run('DELETE FROM sessions WHERE user_id = ?', [$user['id']]);
        Db::run('DELETE FROM user_roles WHERE user_id = ?', [$user['id']]);
        Db::run('DELETE FROM inventory WHERE user_id = ?', [$user['id']]);
        Db::run('DELETE FROM users WHERE id = ?', [$user['id']]);
    }
}

echo "\n" . ($checks - $failures) . " / $checks checks passed\n";
exit($failures === 0 ? 0 : 1);
