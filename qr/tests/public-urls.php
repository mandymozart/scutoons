<?php
/**
 * Printed / public URLs must never change (agents/PLATFORM.md "RULE: printed / public URLs never change").
 * Requests every such URL against the running qr app (qr.scutoons.com) and checks status, content type and redirect
 * target:
 *
 *   /?c=<CODE> and ?c=<CODE> (with and without the leading slash), /?c=<CODE>&skip=1, /?login=<token>,
 *   /verify.html (+ what it loads: /api/certificates/<CODE> and POST /api/certificates/verify – always proxied; the
 *   old page's assets/js/noble-post-quantum.min.js, generate/get-signature.php?code=,
 *   generate/files/signatures/SIGNATURE-<code>.json stay served), /api/validate/ (GET ?code= / ?c=, POST {code} / {c})
 *   and its response shape, /api/keys/keys.php.
 *
 *   php tests/public-urls.php [qr base url] --api=<api base url> [--db=<sqlite file>] [--out=<file>]
 *       [--full-api=on|off] [--down=<qr base url>] [--limited=<qr base url>]
 *
 * --api: the API the qr app proxies to. The proxied answers (validate, keys, verify data, /api/*) must equal the API's
 * own answers byte for byte (status, body, content type, caching, CORS, ETag, Location, Retry-After), and the API must
 * no longer serve the pages. --full-api=off expects a qr app started with PROXY_FULL_API=0 (only the printed API URLs
 * are proxied, the rest of /api/* is 404).
 * --down: a qr app whose API does not answer (same cache directory): the fallback page ("Scanning works, try again in
 * a minute", 503) with Continue for a code this run already looked up, 502 JSON for the proxies.
 * --limited: a qr app in front of an API with a tiny ACCESS_MAX_PER_IP_HOUR: the rate-limited page (429).
 *
 * Codes are picked from the API's SQLite database (default: ../server/db/local.sqlite, via its settings; pass the
 * server's --db when it runs on a copy), one per kind (membership with one option / a choice / no edition, promo, page,
 * event, xr, owned, expired, revoked), ordered by created_at + code so two copies of the same database pick the same
 * codes. --out writes the normalized results (one line per request) so two installations can be compared with `diff`
 * (the recorded baseline of the old server: agents/PLATFORM.md "P3 — result"):
 *
 *   php tests/public-urls.php http://localhost:9001 --api=http://localhost:9000 --out=new.txt && diff baseline.txt new.txt
 *
 * Without a base / --api: the local dev qr app and API (shared/dev-ports.json, `composer test`).
 *
 * Read-only apart from the `scan` events the landing page logs. Exit code 0 = all checks passed.
 * `composer test` in server/ runs it against fresh servers on a copy of the database (server/tests/run-contract.php).
 */
if (PHP_SAPI !== 'cli') {
    http_response_code(404);
    exit;
}

$serverDir = dirname(__DIR__, 2) . '/server';
$devPorts = json_decode((string) @file_get_contents(dirname(__DIR__, 2) . '/shared/dev-ports.json'), true);
$base = is_array($devPorts) ? 'http://localhost:' . $devPorts['qr'] : null;
$api = is_array($devPorts) ? 'http://localhost:' . $devPorts['api'] : null;
$dbFile = null;
$outFile = null;
$fullApi = true;
$downBase = null;
$limitedBase = null;
foreach (array_slice($argv, 1) as $arg) {
    if (str_starts_with($arg, '--db=')) $dbFile = substr($arg, 5);
    elseif (str_starts_with($arg, '--out=')) $outFile = substr($arg, 6);
    elseif (str_starts_with($arg, '--api=')) $api = rtrim(substr($arg, 6), '/');
    elseif (str_starts_with($arg, '--full-api=')) $fullApi = substr($arg, 11) !== 'off';
    elseif (str_starts_with($arg, '--down=')) $downBase = rtrim(substr($arg, 7), '/');
    elseif (str_starts_with($arg, '--limited=')) $limitedBase = rtrim(substr($arg, 10), '/');
    else $base = rtrim($arg, '/');
}
if ($api === null) {
    fwrite(STDERR, "Pass the API the qr app proxies to: --api=http://localhost:9000\n");
    exit(2);
}
if ($dbFile === null) {
    require_once $serverDir . '/vendor/autoload.php';
    $dbFile = Scutoons\Infrastructure\Config\Settings::load($serverDir)->path('DB_SQLITE_PATH');
}
if (!is_file($dbFile)) {
    fwrite(STDERR, "SQLite database not found: $dbFile\n");
    exit(2);
}
$pdo = new PDO('sqlite:' . $dbFile, null, null, [PDO::ATTR_ERRMODE => PDO::ERRMODE_EXCEPTION, PDO::ATTR_DEFAULT_FETCH_MODE => PDO::FETCH_ASSOC]);
$permissionsFile = $serverDir . '/config/permissions.json';
$permissions = json_decode((string) file_get_contents($permissionsFile), true);

$failures = 0;
$checks = 0;
$lines = [];

function check(bool $ok, string $label, mixed $detail = null): void
{
    global $failures, $checks;
    $checks++;
    if ($ok) {
        echo "  PASS  $label\n";
        return;
    }
    $failures++;
    echo "  FAIL  $label" . ($detail !== null ? "\n        " . substr(is_string($detail) ? $detail : json_encode($detail, JSON_UNESCAPED_SLASHES), 0, 400) : '') . "\n";
}

/** @return array{status: int, type: string, location: string, body: string} */
function request(string $method, string $url, ?string $body = null, array $extraHeaders = []): array
{
    $ch = curl_init($url);
    $headers = [];
    curl_setopt_array($ch, [
        CURLOPT_CUSTOMREQUEST => $method,
        CURLOPT_RETURNTRANSFER => true,
        CURLOPT_FOLLOWLOCATION => false,
        CURLOPT_TIMEOUT => 20,
        CURLOPT_HTTPHEADER => array_merge($body !== null ? ['Content-Type: application/json'] : [], $extraHeaders),
        CURLOPT_HEADERFUNCTION => function ($ch, $line) use (&$headers) {
            if (str_contains($line, ':')) {
                [$name, $value] = explode(':', $line, 2);
                $headers[strtolower(trim($name))] = trim($value);
            }
            return strlen($line);
        },
    ]);
    if ($body !== null) curl_setopt($ch, CURLOPT_POSTFIELDS, $body);
    $raw = (string) curl_exec($ch);
    $status = (int) curl_getinfo($ch, CURLINFO_RESPONSE_CODE);
    curl_close($ch);
    $type = strtolower(trim(explode(';', $headers['content-type'] ?? '')[0]));
    return ['status' => $status, 'type' => $type, 'location' => $headers['location'] ?? '', 'body' => $raw, 'headers' => $headers];
}

/** Records the normalized result of a request (for --out) and returns it */
function probe(string $method, string $path, ?string $body = null, string $extra = ''): array
{
    global $base, $lines;
    $r = request($method, $base . $path, $body);
    // Magic-link forwards go to the configured viewer (APP_URL differs per installation): compare path + query only
    $location = str_contains($path, 'login=') ? preg_replace('#^https?://[^/]+#', '<APP_URL>', $r['location']) : $r['location'];
    $lines[] = trim("$method $path" . ($body !== null ? " $body" : '') . " → {$r['status']} {$r['type']}"
        . ($location !== '' ? " → $location" : '') . ($extra !== '' ? " $extra" : ''));
    return $r;
}

/** redirect_to + ?code= / &code= (the landing page's continue URL) */
function continueUrl(string $redirectTo, string $code): string
{
    return $redirectTo . (str_contains($redirectTo, '?') ? '&' : '?') . 'code=' . rawurlencode($code);
}

function typeRules(string $type): array
{
    global $permissions;
    $rules = $permissions['types'][$type] ?? $permissions['default'];
    return ['claimable' => (bool) ($rules['claimable'] ?? false), 'signup' => (bool) ($rules['signup'] ?? false)];
}

// ── Codes, one per kind ─────────────────────────────────────────────────────────────────────────────────
$now = date('Y-m-d H:i:s');
$valid = "revoked_at IS NULL AND (expires_at IS NULL OR expires_at > '$now')";
$pick = function (string $label, string $where) use ($pdo): ?array {
    $row = $pdo->query("SELECT * FROM qr WHERE $where ORDER BY created_at, code LIMIT 1")->fetch();
    return $row ? ['label' => $label] + $row : null;
};
$cases = array_values(array_filter([
    $pick('membership, one option (vinyl)', "type = 'membership' AND edition_id = '3_catpope_vinyl' AND $valid"),
    $pick('membership, choice (dago)', "type = 'membership' AND edition_id = '1_catpope_dago' AND $valid"),
    $pick('membership, beetles', "type = 'membership' AND edition_id = '2_catpope_beetles' AND $valid"),
    $pick('membership, no edition', "type = 'membership' AND (edition_id IS NULL OR edition_id = '') AND $valid"),
    $pick('membership, owned by an account', "type = 'membership' AND claimed_by IS NOT NULL AND $valid"),
    $pick('membership, legacy email claim', "type = 'membership' AND claimed_by IS NULL AND email <> '' AND $valid"),
    $pick('promo', "type = 'promo' AND $valid"),
    $pick('page', "type = 'page' AND $valid"),
    $pick('event', "type = 'event' AND $valid"),
    $pick('xr', "type = 'xr' AND $valid"),
    $pick('expired', "expires_at IS NOT NULL AND expires_at <= '$now'"),
    $pick('revoked', 'revoked_at IS NOT NULL'),
]));
$signed = $pick('signed', "signature IS NOT NULL AND signature <> '' AND $valid");
echo "qr $base · API $api · database $dbFile · " . count($cases) . " kinds of codes\n\n";

function finish(): never
{
    global $checks, $failures, $lines, $outFile;
    if ($outFile !== null) file_put_contents($outFile, implode("\n", $lines) . "\n");
    echo "\n" . ($checks - $failures) . " / $checks checks passed\n";
    exit($failures === 0 ? 0 : 1);
}

// ── QR landing page: ?c= (printed on every QR code), &skip=1, ?login= ───────────────────────────────────
echo "QR landing page\n";
foreach ($cases as $row) {
    $code = $row['code'];
    $rules = typeRules((string) $row['type']);
    $expired = $row['expires_at'] !== null && $row['expires_at'] <= $now;
    $revoked = $row['revoked_at'] !== null;
    foreach (["/?c=$code", "?c=$code"] as $path) {
        $r = probe('GET', $path, null, "[{$row['label']}]");
        if ($expired || $revoked) {
            check($r['status'] === 410 && $r['type'] === 'text/html', "$path ({$row['label']}) → 410 page", $r['status']);
        } elseif ($rules['claimable'] || $rules['signup']) {
            // registered membership codes: Read + "This is mine? File a claim" (claim=1); free ones: register first
            $owned = $row['claimed_by'] !== null || (string) $row['email'] !== '';
            $marker = !$rules['claimable'] ? 'Create account' : ($owned ? 'id="file-claim"' : 'Register it now');
            check($r['status'] === 200 && $r['type'] === 'text/html' && str_contains($r['body'], $marker)
                && str_contains($r['body'], htmlspecialchars(continueUrl($row['redirect_to'], $code), ENT_QUOTES, 'UTF-8')),
                "$path ({$row['label']}) → 200 page with Continue + $marker", [$r['status'], $r['type']]);
        } else {
            check($r['status'] === 302 && $r['location'] === continueUrl($row['redirect_to'], $code),
                "$path ({$row['label']}) → 302 to redirect_to?code=", [$r['status'], $r['location']]);
        }
    }
    $r = probe('GET', "/?c=$code&skip=1", null, "[{$row['label']}]");
    if ($expired || $revoked) {
        check($r['status'] === 410, "/?c=…&skip=1 ({$row['label']}) → 410", $r['status']);
    } else {
        check($r['status'] === 302 && $r['location'] === continueUrl($row['redirect_to'], $code),
            "/?c=…&skip=1 ({$row['label']}) → 302 to redirect_to?code=", [$r['status'], $r['location']]);
    }
}
$r = probe('GET', '/?c=zzzzzzzz');
check($r['status'] === 404 && $r['type'] === 'text/html', '/?c=<unknown> → 404 page', $r['status']);
$r = probe('GET', '?c=zzzzzzzz&skip=1');
check($r['status'] === 404, '?c=<unknown>&skip=1 → 404 page', $r['status']);
$r = probe('GET', '/');
check($r['status'] === 200 && $r['type'] === 'text/html' && str_contains($r['body'], 'name="c"'), '/ → code entry form (param c)', $r['status']);
$r = probe('GET', '/?login=abcdefghijklmnopqrstuvwxyz');
check($r['status'] === 302 && str_ends_with($r['location'], '/?login=abcdefghijklmnopqrstuvwxyz'), '/?login= → 302 to the viewer', $r['location']);

// ── Verify page and what it loads ──────────────────────────────────────────────────────────────────────
echo "\nVerify page\n";
foreach (['/verify.html', '/verify.html?code=' . ($signed['code'] ?? 'x')] as $path) {
    $r = probe('GET', $path);
    $lines[count($lines) - 1] .= ' body sha1:' . sha1($r['body']);
    check($r['status'] === 200 && $r['type'] === 'text/html' && str_contains($r['body'], 'api/certificates/'),
        "$path → 200, verifies through /api/certificates/", $r['status']);
}
if ($signed) {
    // codes are case-sensitive and matched exactly (owner 2026-10-03): the exact code verifies, another case is unknown
    $r = probe('GET', '/api/certificates/' . rawurlencode($signed['code']));
    $body = json_decode($r['body'], true);
    check($r['status'] === 200 && ($body['valid'] ?? null) === true && ($body['code'] ?? null) === $signed['code']
        && is_bool($body['registered'] ?? null) && !str_contains($r['body'], '@'),
        '/api/certificates/<signed code> → authentic, no owner data', [$r['status'], substr($r['body'], 0, 200)]);
    $other = strtoupper($signed['code']) !== $signed['code'] ? strtoupper($signed['code']) : strtolower($signed['code']);
    if ($other !== $signed['code'] && (int) $pdo->query('SELECT COUNT(*) FROM qr WHERE code = ' . $pdo->quote($other))->fetchColumn() === 0) {
        $r = probe('GET', '/api/certificates/' . rawurlencode($other));
        check($r['status'] === 200 && (json_decode($r['body'], true)['reason'] ?? null) === 'unknown_code',
            '/api/certificates/<signed code in another case> → unknown_code (no case-insensitive match)', [$r['status'], substr($r['body'], 0, 200)]);
    }
}
$r = probe('GET', '/api/certificates/zzzzzzzz');
check($r['status'] === 200 && (json_decode($r['body'], true)['reason'] ?? null) === 'unknown_code', '/api/certificates/<unknown> → unknown_code', [$r['status'], $r['body']]);
$r = probe('GET', '/assets/js/noble-post-quantum.min.js');
$lines[count($lines) - 1] .= ' body sha1:' . sha1($r['body']);
check($r['status'] === 200 && str_contains($r['type'], 'javascript'), 'assets/js/noble-post-quantum.min.js (verify page library)', [$r['status'], $r['type']]);
$r = probe('GET', '/assets/js/verify-crypto.js');
check($r['status'] === 200 && str_contains($r['type'], 'javascript'), 'assets/js/verify-crypto.js', [$r['status'], $r['type']]);
if ($signed) {
    $r = probe('GET', '/generate/get-signature.php?code=' . rawurlencode($signed['code']));
    $body = json_decode($r['body'], true);
    check($r['status'] === 200 && ($body['success'] ?? null) === true && isset($body['signature']['signature'], $body['signature']['publicKey'], $body['signature']['signedData']),
        'generate/get-signature.php?code=<signed code> → signature (verify page)', [$r['status'], substr($r['body'], 0, 200)]);
    $lines[] = 'get-signature keys: ' . implode(',', array_keys((array) $body));
}
$r = probe('GET', '/generate/get-signature.php?code=zzzzzzzz');
check($r['status'] === 404, 'generate/get-signature.php?code=<unknown> → 404 (verify page falls back to the file)', $r['status']);
$signatureFiles = glob($serverDir . '/{storage/files,generate/files}/signatures/SIGNATURE-*.json', GLOB_BRACE) ?: [];
sort($signatureFiles);
if ($signatureFiles) {
    $name = basename($signatureFiles[0]);
    $r = probe('GET', "/generate/files/signatures/$name");
    check($r['status'] === 200 && is_array(json_decode($r['body'], true)), "generate/files/signatures/$name (verify page fallback)", $r['status']);
}

// ── /api/validate/ (old viewer builds, external links) ─────────────────────────────────────────────────
echo "\n/api/validate/\n";
$shape = function (array $r): string {
    $body = json_decode($r['body'], true);
    if (!is_array($body)) return 'not json';
    $keys = array_keys($body);
    sort($keys);
    $qrKeys = is_array($body['qr'] ?? null) ? array_keys($body['qr']) : [];
    sort($qrKeys);
    return 'keys:' . implode(',', $keys) . ($qrKeys ? ' qr:' . implode(',', $qrKeys) : '') . ' status:' . ($body['status'] ?? '');
};
$member = $cases[0];
foreach ([
    ['GET', '/api/validate/?code=' . $member['code'], null],
    ['GET', '/api/validate/?c=' . $member['code'], null],
    ['POST', '/api/validate/', json_encode(['code' => $member['code']])],
    ['POST', '/api/validate/', json_encode(['c' => $member['code']])],
    ['GET', '/api/validate?c=' . $member['code'], null],
    ['GET', '/api/validate/index.php?c=' . $member['code'], null],
] as [$method, $path, $body]) {
    $r = request($method, $base . $path, $body);
    $lines[] = "$method $path" . ($body !== null ? " $body" : '') . " → {$r['status']} {$r['type']} " . $shape($r);
    $decoded = json_decode($r['body'], true);
    check($r['status'] === 200 && $r['type'] === 'application/json' && ($decoded['status'] ?? null) === 'success'
        && ($decoded['qr']['code'] ?? null) === $member['code'] && !array_key_exists('email', $decoded['qr'] ?? [])
        && !array_key_exists('signature', $decoded['qr'] ?? []) && !array_key_exists('claimed_by', $decoded['qr'] ?? [])
        && is_bool($decoded['qr']['is_claimed'] ?? null) && ($decoded['qr']['is_expired'] ?? null) === false,
        "$method $path" . ($body !== null ? " $body" : '') . ' → success + qr (no email/signature/claimed_by)', [$r['status'], substr($r['body'], 0, 200)]);
}
foreach ([['GET', '/api/validate/?c=zzzzzzzz', null, 404], ['GET', '/api/validate/', null, 400], ['POST', '/api/validate/', '{}', 400]] as [$method, $path, $body, $status]) {
    $r = request($method, $base . $path, $body);
    $lines[] = "$method $path" . ($body !== null ? " $body" : '') . " → {$r['status']} {$r['type']} " . $shape($r);
    check($r['status'] === $status && (json_decode($r['body'], true)['status'] ?? null) === 'error', "$method $path → $status error", [$r['status'], $r['body']]);
}
foreach ($cases as $row) {
    if ($row['expires_at'] !== null && $row['expires_at'] <= $now) {
        $r = request('GET', $base . '/api/validate/?c=' . $row['code']);
        $lines[] = "GET /api/validate/?c=<expired> → {$r['status']} {$r['type']} " . $shape($r);
        check($r['status'] === 410, 'validate expired → 410', $r['status']);
    }
}

// ── Public key ─────────────────────────────────────────────────────────────────────────────────────────
echo "\n/api/keys/keys.php\n";
$r = request('GET', "$base/api/keys/keys.php");
$body = json_decode($r['body'], true);
$lines[] = "GET /api/keys/keys.php → {$r['status']} {$r['type']} " . $shape($r) . (isset($body['publicKey']) ? ' publicKey sha1:' . sha1((string) $body['publicKey']) : '');
check($r['status'] === 200 && $r['type'] === 'application/json' && ($body['success'] ?? null) === true && is_string($body['publicKey'] ?? null)
    && !array_key_exists('privateKey', $body), '/api/keys/keys.php → public key only', [$r['status'], substr($r['body'], 0, 100)]);

// ── The qr app: proxies are byte-identical to the API, the API has no pages any more ────────────────────
// (not written to --out: the old server had no separate API to compare with)
echo "\nqr app → API ($api)" . ($fullApi ? '' : ', PROXY_FULL_API off') . "\n";
/** The headers a client can notice (everything else is transport: Date, Server, Connection, lengths) */
function relevantHeaders(array $r): array
{
    $keep = [];
    foreach ($r['headers'] as $name => $value) {
        if (in_array($name, ['content-type', 'cache-control', 'etag', 'last-modified', 'location', 'retry-after', 'vary',
            'content-disposition', 'allow'], true) || str_starts_with($name, 'access-control-')) $keep[$name] = $value;
    }
    ksort($keep);
    return $keep;
}
$signedCode = $signed['code'] ?? $member['code'];
$identical = [
    ['GET', '/api/validate/?c=' . $member['code'], null, []],
    ['GET', '/api/validate/?code=' . $member['code'], null, ['Origin: https://scutoons.com']],
    ['POST', '/api/validate/', json_encode(['c' => $member['code']]), []],
    ['GET', '/api/validate/index.php?c=' . $member['code'], null, []],
    ['GET', '/api/validate?c=zzzzzzzz', null, []],
    ['GET', '/api/validate/', null, []],
    ['OPTIONS', '/api/validate/', null, ['Origin: https://example.org', 'Access-Control-Request-Method: POST']],
    ['PUT', '/api/validate/', '{}', []],
    ['GET', '/api/keys/keys.php', null, []],
    ['GET', '/generate/get-signature.php?code=' . rawurlencode($signedCode), null, []],
    ['GET', '/generate/get-signature.php?code=zzzzzzzz', null, []],
    ['GET', '/api/certificates/' . rawurlencode($signedCode), null, ['Origin: https://example.org']],
    ['GET', '/api/certificates/zzzzzzzz', null, []],
    ['POST', '/api/certificates/verify', '{}', []],
    ['OPTIONS', '/api/certificates/verify', null, ['Origin: https://example.org', 'Access-Control-Request-Method: POST']],
];
if ($signatureFiles ?? []) $identical[] = ['GET', '/generate/files/signatures/' . basename($signatureFiles[0]), null, []];
if ($fullApi) {
    array_push($identical,
        ['GET', '/api/health/', null, []],
        ['GET', '/api/items/', null, ['Origin: https://scutoons.com']],
        ['GET', '/api/products/', null, []],
        ['GET', '/api/me/', null, ['Origin: https://scutoons.com']],
        ['GET', '/api/me/', null, ['Authorization: Bearer not-a-session', 'Origin: http://localhost:8000']],
        ['OPTIONS', '/api/me/', null, ['Origin: https://scutoons.com', 'Access-Control-Request-Method: GET', 'Access-Control-Request-Headers: authorization,x-device-id']],
        ['GET', '/api/code/?c=' . $member['code'], null, []],
        ['POST', '/api/auth/request/', '{bad', []],
        ['GET', '/api/nope/', null, []],
    );
}
foreach ($identical as [$method, $path, $body, $headers]) {
    $viaQr = request($method, $base . $path, $body, $headers);
    $direct = request($method, $api . $path, $body, $headers);
    $same = $viaQr['status'] === $direct['status'] && $viaQr['body'] === $direct['body'] && relevantHeaders($viaQr) === relevantHeaders($direct);
    check($same && $viaQr['status'] !== 502, "proxy $method $path → {$direct['status']} identical to the API", [
        'qr' => [$viaQr['status'], relevantHeaders($viaQr), substr($viaQr['body'], 0, 120)],
        'api' => [$direct['status'], relevantHeaders($direct), substr($direct['body'], 0, 120)],
    ]);
}
if ($fullApi) {
    // Single sign-on during the transition (apps call qr.scutoons.com/api): the proxy forwards the session cookie
    // scutoons_sid – and only it – so cookie sign-in works through qr too. A throw-away session row, removed again.
    $user = $pdo->query('SELECT id, email FROM users ORDER BY created_at, id LIMIT 1')->fetch();
    if ($user) {
        $token = 'qrproxy' . bin2hex(random_bytes(16));
        $sessionId = 'qr-proxy-test-' . bin2hex(random_bytes(6));
        $now = gmdate('Y-m-d H:i:s');
        $pdo->prepare('INSERT INTO sessions (id, user_id, token_hash, created_at, last_used_at) VALUES (?, ?, ?, ?, ?)')
            ->execute([$sessionId, $user['id'], hash('sha256', $token), $now, $now]);
        try {
            $r = request('GET', $base . '/api/auth/session/', null, ["Cookie: theme=dark; scutoons_sid=$token", 'Origin: http://localhost:8000']);
            check($r['status'] === 200 && (json_decode($r['body'], true)['user']['email'] ?? null) === $user['email'],
                'proxy forwards the single sign-on cookie scutoons_sid (GET /api/auth/session/ → the user)', [$r['status'], substr($r['body'], 0, 160)]);
            $r = request('GET', $base . '/api/auth/session/', null, ["Cookie: other=$token"]);
            $answer = json_decode($r['body'], true);
            check($r['status'] === 200 && is_array($answer) && array_key_exists('user', $answer) && $answer['user'] === null,
                'proxy drops every other cookie', [$r['status'], substr($r['body'], 0, 120)]);
        } finally {
            $pdo->prepare('DELETE FROM sessions WHERE id = ?')->execute([$sessionId]);
        }
    }
}
if (!$fullApi) {
    foreach (['/api/health/', '/api/me/', '/api/items/', '/api/code/?c=' . $member['code']] as $path) {
        $r = request('GET', $base . $path);
        check($r['status'] === 404 && (json_decode($r['body'], true)['error'] ?? null) === 'not_found', "PROXY_FULL_API off: GET $path → 404 not_found", [$r['status'], $r['body']]);
    }
}
$r = request('GET', $base . '/generate/?x=1');
check($r['status'] === 302 && $r['location'] === $api . '/generate/?x=1', 'legacy generate/ → 302 to the API', [$r['status'], $r['location']]);
$r = request('GET', $base . '/version.json');
$version = json_decode($r['body'], true);
check($r['status'] === 200 && ($version['app'] ?? null) === 'qr' && is_string($version['version'] ?? null) && isset($version['git_sha']), '/version.json → { app: qr, version, git_sha }', [$r['status'], $r['body']]);
foreach (['/?c=' . $member['code'], '/verify.html', '/', '/?login=abc'] as $path) {
    $r = request('GET', $api . $path);
    check($r['status'] === 404, "API: GET $path → 404 (the pages are the qr app)", $r['status']);
}
$r = request('GET', $api . '/api/health/');
$health = json_decode($r['body'], true);
check($r['status'] === 200 && is_string($health['version'] ?? null) && is_string($health['git_sha'] ?? null), 'API: /api/health/ → version + git_sha', $r['body']);

// ── API down: the fallback page (a code of this run is cached) ──────────────────────────────────────────
if ($downBase !== null) {
    echo "\nAPI down ($downBase)\n";
    $r = request('GET', $downBase . '/?c=' . $member['code']);
    check($r['status'] === 503 && str_contains($r['body'], 'Scanning works, try again in a minute')
        && str_contains($r['body'], htmlspecialchars(continueUrl($member['redirect_to'], $member['code']), ENT_QUOTES, 'UTF-8')),
        'cached code → 503 "Scanning works, try again in a minute" + Continue to redirect_to?code=', [$r['status'], substr($r['body'], -600)]);
    $r = request('GET', $downBase . '/?c=' . $member['code'] . '&skip=1');
    check(in_array($r['status'], [302, 503], true), 'cached code &skip=1 → redirect (fresh cache) or fallback page', $r['status']);
    $r = request('GET', $downBase . '/?c=neverSeen1');
    check($r['status'] === 503 && !str_contains($r['body'], 'id="continue"') && ($r['headers']['retry-after'] ?? '') === '60', 'uncached code → 503 without Continue, Retry-After 60', [$r['status'], $r['headers']]);
    $r = request('GET', $downBase . '/api/validate/?c=' . $member['code']);
    check($r['status'] === 502 && (json_decode($r['body'], true)['error'] ?? null) === 'api_unavailable', 'proxy → 502 api_unavailable', [$r['status'], $r['body']]);
    $r = request('GET', $downBase . '/verify.html');
    check($r['status'] === 200, 'verify page is static: still 200', $r['status']);
}

// ── Rate limit of the API (per client IP, forwarded by the qr app) ──────────────────────────────────────
if ($limitedBase !== null) {
    echo "\nRate limit ($limitedBase)\n";
    $limited = null;
    for ($i = 0; $i < 6 && $limited === null; $i++) {
        $r = request('GET', $limitedBase . '/?c=' . $member['code']);
        if ($r['status'] === 429) $limited = $r;
    }
    check($limited !== null && $limited['type'] === 'text/html' && str_contains($limited['body'], 'Too many codes checked'), 'too many lookups → 429 page', $limited['status'] ?? null);
}
finish();
