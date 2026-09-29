<?php
/**
 * Printed / public URLs must never change (agents/PLATFORM.md "RULE: printed / public URLs never change").
 * Requests every such URL against a running server and checks status, content type and redirect target:
 *
 *   /?c=<CODE> and ?c=<CODE> (with and without the leading slash), /?c=<CODE>&skip=1, /?login=<token>,
 *   /verify.html (+ what it loads: assets/js/noble-post-quantum.min.js, generate/get-signature.php?code=,
 *   generate/files/signatures/SIGNATURE-<code>.json), /api/validate/ (GET ?code= / ?c=, POST {code} / {c}) and its
 *   response shape, /api/keys/keys.php.
 *
 *   php tests/public-urls.php [base url] [--db=<sqlite file>] [--out=<file>]
 *
 * Codes are picked from the server's SQLite database (default: the configured DB_SQLITE_PATH; for another server pass
 * its --db), one per kind (membership with one option / a choice / no edition, promo, page, event, xr, owned,
 * expired, revoked), ordered by created_at + code so two copies of the same database pick the same codes.
 * --out writes the normalized results (one line per request) so two servers can be compared with `diff`:
 *
 *   php tests/public-urls.php http://localhost:9000 --db=<old repo>/db/local.sqlite --out=old.txt
 *   php tests/public-urls.php http://localhost:9100 --out=new.txt && diff old.txt new.txt
 *
 * Read-only apart from the `scan` events the landing page logs. Exit code 0 = all checks passed.
 */
if (PHP_SAPI !== 'cli') {
    http_response_code(404);
    exit;
}

$base = 'http://localhost:9100';
$dbFile = null;
$outFile = null;
foreach (array_slice($argv, 1) as $arg) {
    if (str_starts_with($arg, '--db=')) $dbFile = substr($arg, 5);
    elseif (str_starts_with($arg, '--out=')) $outFile = substr($arg, 6);
    else $base = rtrim($arg, '/');
}
if ($dbFile === null) {
    require_once __DIR__ . '/../vendor/autoload.php';
    $dbFile = Scutoons\Infrastructure\Config\Settings::load()->path('DB_SQLITE_PATH');
}
if (!is_file($dbFile)) {
    fwrite(STDERR, "SQLite database not found: $dbFile\n");
    exit(2);
}
$pdo = new PDO('sqlite:' . $dbFile, null, null, [PDO::ATTR_ERRMODE => PDO::ERRMODE_EXCEPTION, PDO::ATTR_DEFAULT_FETCH_MODE => PDO::FETCH_ASSOC]);
$permissionsFile = is_file(__DIR__ . '/../config/permissions.json') ? __DIR__ . '/../config/permissions.json' : __DIR__ . '/../api/permissions.json';
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
function request(string $method, string $url, ?string $body = null): array
{
    $ch = curl_init($url);
    $headers = [];
    curl_setopt_array($ch, [
        CURLOPT_CUSTOMREQUEST => $method,
        CURLOPT_RETURNTRANSFER => true,
        CURLOPT_FOLLOWLOCATION => false,
        CURLOPT_TIMEOUT => 20,
        CURLOPT_HTTPHEADER => $body !== null ? ['Content-Type: application/json'] : [],
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
    return ['status' => $status, 'type' => $type, 'location' => $headers['location'] ?? '', 'body' => $raw];
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
echo "Server $base · database $dbFile · " . count($cases) . " kinds of codes\n\n";

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
            $marker = $rules['claimable'] ? 'Add to my account' : 'Create account';
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
    check($r['status'] === 200 && $r['type'] === 'text/html' && str_contains($r['body'], 'generate/get-signature.php?code=')
        && str_contains($r['body'], 'assets/js/noble-post-quantum.min.js'), "$path → 200, loads signature by code", $r['status']);
}
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
$signatureFiles = glob(dirname(__DIR__) . '/{storage/files,generate/files}/signatures/SIGNATURE-*.json', GLOB_BRACE) ?: [];
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

if ($outFile !== null) file_put_contents($outFile, implode("\n", $lines) . "\n");
echo "\n" . ($checks - $failures) . " / $checks checks passed\n";
exit($failures === 0 ? 0 : 1);
