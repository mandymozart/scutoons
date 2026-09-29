<?php
/**
 * End-to-end smoke test of the accounts/items API against a running local server.
 *
 *   php -S localhost:9000 router.php          (MAIL_TRANSPORT = log in config.local.php)
 *   php tests/api-smoke.php [http://localhost:9000]
 *
 * Uses fresh email addresses per run and picks unclaimed codes from the local DB (db/local.sqlite): it claims
 * one vinyl code (single option) and one Dago code (choice) – run it against a local copy only.
 * OTPs are read from the mail log (logs/mail.log). Exit code 0 = all checks passed.
 */
if (PHP_SAPI !== 'cli') {
    http_response_code(404);
    exit;
}

require_once __DIR__ . '/../api/lib/bootstrap.php';

$base = rtrim($argv[1] ?? 'http://localhost:9000', '/');
$api = $base . '/api';
$mailLog = Config::get('MAIL_LOG_PATH');
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

/** @return array{status: int, body: mixed, headers: array<string, string>} */
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
    if ($body !== null) curl_setopt($ch, CURLOPT_POSTFIELDS, is_string($body) ? $body : json_encode($body));
    $raw = curl_exec($ch);
    $status = (int) curl_getinfo($ch, CURLINFO_RESPONSE_CODE);
    curl_close($ch);
    return ['status' => $status, 'body' => json_decode((string) $raw, true) ?? $raw, 'headers' => $responseHeaders];
}

/** The newest one-time code mailed to $email */
function otpFor(string $email): ?string
{
    global $mailLog;
    clearstatcache();
    $log = (string) @file_get_contents($mailLog);
    $pos = strrpos($log, "To: $email\r\n");
    if ($pos === false) return null;
    return preg_match('/^\s{4}(\d{6})\s*$/m', substr($log, $pos), $m) ? $m[1] : null;
}

function login(string $email, ?array $request = null, ?array $verifyExtra = null): array
{
    global $api;
    $r = call('POST', "$api/auth/request/", ['email' => $email, 'locale' => 'en'] + ($request ?? []));
    check($r['status'] === 200 && ($r['body']['status'] ?? null) === 'sent', "auth/request $email → sent", $r);
    $otp = otpFor($email);
    check($otp !== null, "OTP for $email found in mail log");
    $v = call('POST', "$api/auth/verify/", ['email' => $email, 'otp' => $otp] + ($verifyExtra ?? []));
    check($v['status'] === 200 && is_string($v['body']['session_token'] ?? null), "auth/verify $email → session", $v);
    return $v['body'];
}

function entries(array $inventory, string $itemId): array
{
    return array_values(array_filter($inventory, fn($e) => $e['item_id'] === $itemId));
}

function qty(array $inventory, string $itemId): int
{
    return array_sum(array_column(entries($inventory, $itemId), 'qty'));
}

// ── Pick unclaimed local codes ─────────────────────────────────────────────────────────────────────────
$pick = fn(string $edition) => Db::value(
    "SELECT code FROM qr WHERE edition_id = ? AND claimed_by IS NULL AND email = '' AND edition_number IS NOT NULL AND (expires_at IS NULL OR expires_at > ?) ORDER BY edition_number DESC LIMIT 1",
    [$edition, qrNow()]);
$vinyl = $pick('3_catpope_vinyl');
$dago = $pick('1_catpope_dago');
if (!$vinyl || !$dago) {
    fwrite(STDERR, "No unclaimed vinyl/dago codes left in the local DB.\n");
    exit(2);
}
$run = gmdate('YmdHis') . '-' . bin2hex(random_bytes(2));
$emailA = "smoke-$run-a@example.com";
$emailB = "smoke-$run-b@example.com";
echo "API $api · codes vinyl=$vinyl dago=$dago · users $emailA, $emailB\n\n";

// ── Public endpoints ───────────────────────────────────────────────────────────────────────────────────
echo "Public\n";
$r = call('GET', "$api/health/");
check($r['status'] === 200 && $r['body']['db'] === 'ok', 'GET /health/', $r);
$r = call('GET', "$api/items/");
check($r['status'] === 200 && count($r['body']['items'] ?? []) === 9 && ($r['body']['recipes'][0]['id'] ?? '') === 'cucumber_camera', 'GET /items/ → 9 items + recipe', $r['status']);
$r = call('GET', "$api/code/?c=" . rawurlencode($dago));
check($r['status'] === 200 && $r['body']['claimed'] === false && count($r['body']['options']) === 2
    && $r['body']['edition']['count'] === 37 && is_int($r['body']['edition']['number']), 'GET /code/ dago → 2 options, edition n / 37', $r);
check(!array_key_exists('email', (array) $r['body']), 'GET /code/ has no email');
$r = call('GET', "$api/code/?c=zzzzzzzz");
check($r['status'] === 404 && $r['body']['error'] === 'not_found', 'GET /code/ unknown → 404', $r);
$r = call('OPTIONS', "$api/me/", null, null, ['Origin: http://localhost:5173', 'Access-Control-Request-Method: GET']);
check($r['status'] === 204 && ($r['headers']['access-control-allow-origin'] ?? '') === 'http://localhost:5173', 'CORS preflight from localhost:5173', $r['headers']);
$r = call('OPTIONS', "$api/me/", null, null, ['Origin: https://evil.example', 'Access-Control-Request-Method: GET']);
check(!isset($r['headers']['access-control-allow-origin']), 'CORS: other origins get no allow header', $r['headers']);
$r = call('GET', "$api/me/");
check($r['status'] === 401 && $r['body']['error'] === 'unauthorized', 'GET /me/ without token → 401', $r);

// ── Login + claim in the same step ─────────────────────────────────────────────────────────────────────
echo "\nLogin (user A, with vinyl code)\n";
$r = call('POST', "$api/auth/request/", ['email' => 'not-an-email']);
check($r['status'] === 400 && $r['body']['error'] === 'invalid_email', 'auth/request invalid email → 400', $r);
call('POST', "$api/auth/request/", ['email' => $emailA, 'code' => $vinyl, 'locale' => 'en']);
$wrong = otpFor($emailA) === '000000' ? '111111' : '000000';
$r = call('POST', "$api/auth/verify/", ['email' => $emailA, 'otp' => $wrong]);
check($r['status'] === 400 && $r['body']['error'] === 'invalid_otp', 'auth/verify wrong OTP → 400 invalid_otp', $r);
$otp = otpFor($emailA);
check($otp !== null, 'OTP for user A found in mail log');
$v = call('POST', "$api/auth/verify/", ['email' => $emailA, 'otp' => $otp]);
check($v['status'] === 200 && $v['body']['created'] === true && $v['body']['claimed'] === [$vinyl], 'auth/verify → new user, vinyl code claimed', $v);
$tokenA = $v['body']['session_token'];
$r = call('POST', "$api/auth/verify/", ['email' => $emailA, 'otp' => $otp]);
check($r['status'] === 410 && $r['body']['error'] === 'already_used', 'auth/verify same OTP again → 410 already_used', $r);

$me = call('GET', "$api/me/", null, $tokenA);
check($me['status'] === 200 && $me['body']['user']['email'] === $emailA, 'GET /me/', $me);
$inv = $me['body']['inventory'];
check(qty($inv, 'camera') === 1 && entries($inv, 'camera')[0]['source'] === 'starter', 'starter camera granted', $inv);
$vinylEntry = entries($inv, 'awakening_vinyl')[0] ?? null;
check($vinylEntry && $vinylEntry['code'] === $vinyl && $vinylEntry['source'] === 'qr' && is_int($vinylEntry['edition_number']), 'vinyl in inventory with code + edition_number', $vinylEntry);
check(in_array($vinyl, array_column($me['body']['memberships'], 'code'), true), 'memberships list the vinyl code');
check(array_keys((array) $me['body']['avatar']['slots']) === Catalog::slots(), 'avatar lists all slots');

// ── Claim with a choice ────────────────────────────────────────────────────────────────────────────────
echo "\nClaim (dago code: choice needed)\n";
$r = call('POST', "$api/inventory/claim/", ['code' => $dago], $tokenA);
check($r['status'] === 422 && $r['body']['error'] === 'choose_item' && count($r['body']['options']) === 2, 'claim without choice → 422 choose_item + options', $r);
$r = call('POST', "$api/inventory/claim/", ['code' => $dago, 'item_id' => 'dago_hoodie', 'variant' => 'pink'], $tokenA);
check($r['status'] === 422 && $r['body']['error'] === 'invalid_item', 'claim with unknown variant → 422 invalid_item', $r);
$r = call('POST', "$api/inventory/claim/", ['code' => $dago, 'item_id' => 'dago_hoodie', 'variant' => 'black'], $tokenA);
check($r['status'] === 200 && $r['body']['claimed'] === 'new', 'claim dago_hoodie black → new', $r);
$hoodie = entries($r['body']['inventory'] ?? [], 'dago_hoodie')[0] ?? null;
check($hoodie && $hoodie['variant'] === 'black' && $hoodie['code'] === $dago, 'hoodie entry with variant + code', $hoodie);
$count = count($r['body']['inventory'] ?? []);
$r = call('POST', "$api/inventory/claim/", ['code' => $dago, 'item_id' => 'dago_heavy_t', 'variant' => 'white'], $tokenA);
check($r['status'] === 200 && $r['body']['claimed'] === 'already_yours' && count($r['body']['inventory']) === $count, 'claim again by same user → idempotent (already_yours)', $r);
$r = call('GET', "$api/code/?c=" . rawurlencode($dago), null, $tokenA);
check($r['status'] === 200 && $r['body']['claimed'] === true && $r['body']['claimed_by_me'] === true, 'GET /code/ as owner → claimed_by_me', $r);

// ── Second user ────────────────────────────────────────────────────────────────────────────────────────
echo "\nUser B\n";
$b = login($emailB);
$tokenB = $b['session_token'];
$r = call('POST', "$api/inventory/claim/", ['code' => $dago, 'item_id' => 'dago_hoodie', 'variant' => 'black'], $tokenB);
check($r['status'] === 409 && $r['body']['error'] === 'already_claimed', 'claim by another user → 409 already_claimed', $r);
$r = call('GET', "$api/code/?c=" . rawurlencode($dago), null, $tokenB);
check($r['status'] === 200 && $r['body']['claimed_by_me'] === false, 'GET /code/ as other user → claimed_by_me false', $r);

// ── Grants ─────────────────────────────────────────────────────────────────────────────────────────────
echo "\nGrant / craft\n";
$ref = "smoke:$run:ch1";
$r = call('POST', "$api/inventory/grant/", ['item_id' => 'cucumber', 'source' => 'story', 'ref' => $ref], $tokenA);
check($r['status'] === 200 && $r['body']['granted'] === 1 && qty($r['body']['inventory'], 'cucumber') === 1, 'grant cucumber (story) → 1 cucumber', $r);
$r = call('POST', "$api/inventory/grant/", ['item_id' => 'cucumber', 'source' => 'story', 'ref' => $ref], $tokenA);
check($r['status'] === 200 && $r['body']['granted'] === 0 && qty($r['body']['inventory'], 'cucumber') === 1, 'same ref again → granted 0', $r);
$r = call('POST', "$api/inventory/grant/", ['item_id' => 'camera', 'source' => 'story'], $tokenA);
check($r['status'] === 403 && $r['body']['error'] === 'not_grantable', 'grant camera → 403 not_grantable', $r);
$r = call('POST', "$api/inventory/grant/", ['item_id' => 'cucumber', 'source' => 'scan'], $tokenA);
check($r['status'] === 403 && $r['body']['error'] === 'no_tool', 'scan grant without cucumber camera → 403 no_tool', $r);

// ── Craft ──────────────────────────────────────────────────────────────────────────────────────────────
$r = call('POST', "$api/inventory/craft/", ['recipe_id' => 'cucumber_camera'], $tokenA);
$inv = $r['body']['inventory'] ?? [];
check($r['status'] === 200 && qty($inv, 'cucumber_camera') === 1 && qty($inv, 'camera') === 0 && qty($inv, 'cucumber') === 0,
    'craft cucumber_camera → camera + cucumber consumed', $r);
$r = call('POST', "$api/inventory/craft/", ['recipe_id' => 'cucumber_camera'], $tokenA);
check($r['status'] === 422 && $r['body']['error'] === 'missing_inputs', 'craft again → 422 missing_inputs', $r);
$r = call('POST', "$api/inventory/grant/", ['item_id' => 'cucumber', 'source' => 'scan'], $tokenA);
check($r['status'] === 200 && $r['body']['granted'] === 1, 'scan grant with cucumber camera → 1', $r);
$r = call('POST', "$api/inventory/grant/", ['item_id' => 'cucumber', 'source' => 'scan'], $tokenA);
check($r['status'] === 429 && $r['body']['error'] === 'cooldown', 'scan grant again at once → 429 cooldown', $r);

// ── Avatar ─────────────────────────────────────────────────────────────────────────────────────────────
echo "\nAvatar\n";
$r = call('PUT', "$api/avatar/", ['slots' => ['torso' => $hoodie['id']]], $tokenA);
check($r['status'] === 200 && ($r['body']['avatar']['slots']['torso'] ?? null) === $hoodie['id'], 'equip hoodie on torso', $r);
$r = call('PUT', "$api/avatar/", ['slots' => ['head' => $hoodie['id']]], $tokenA);
check($r['status'] === 422 && $r['body']['error'] === 'wrong_slot', 'hoodie on head → 422 wrong_slot', $r);
$r = call('PUT', "$api/avatar/", ['slots' => ['tail' => null]], $tokenA);
check($r['status'] === 422 && $r['body']['error'] === 'invalid_slot', 'unknown slot → 422 invalid_slot', $r);
$r = call('PUT', "$api/avatar/", ['slots' => ['torso' => $hoodie['id']]], $tokenB);
check($r['status'] === 422 && $r['body']['error'] === 'not_owned', "equip someone else's entry → 422 not_owned", $r);
$me = call('GET', "$api/me/", null, $tokenA);
check(($me['body']['avatar']['slots']['torso'] ?? null) === $hoodie['id'], '/me/ shows the worn hoodie');

// ── Progress ───────────────────────────────────────────────────────────────────────────────────────────
echo "\nProgress\n";
$r = call('GET', "$api/progress/catpope/", null, $tokenA);
check($r['status'] === 200 && $r['body']['state'] === null && $r['body']['updated_at'] === null, 'GET progress (empty)', $r);
$r = call('PUT', "$api/progress/catpope/", ['state' => ['cucumber' => 2, 'flags' => new stdClass()], 'updated_at' => 2000], $tokenA);
check($r['status'] === 200 && $r['body']['status'] === 'success' && $r['body']['updated_at'] === 2000, 'PUT progress (updated_at 2000)', $r);
$r = call('PUT', "$api/progress/catpope/", ['state' => ['cucumber' => 1], 'updated_at' => 1000], $tokenA);
check($r['status'] === 200 && $r['body']['status'] === 'stale' && $r['body']['state']['cucumber'] === 2, 'PUT older state → stale, newer kept', $r);
$raw = call('GET', "$api/progress/catpope/", null, $tokenA);
check($raw['body']['state']['cucumber'] === 2 && $raw['body']['updated_at'] === 2000, 'GET progress → newest state', $raw);
$r = call('PUT', "$api/progress/catpope/", ['state' => ['cucumber' => 3], 'updated_at' => 3000], $tokenA);
check($r['body']['status'] === 'success' && $r['body']['state']['cucumber'] === 3, 'PUT newer state → success', $r);
$r = call('GET', "$api/progress/catpope/", null, $tokenB);
check($r['body']['state'] === null, 'progress is per user');

// ── Import (guest inventory) ───────────────────────────────────────────────────────────────────────────
echo "\nImport\n";
$r = call('POST', "$api/inventory/import/", ['items' => [['item_id' => 'cucumber', 'qty' => 3], ['item_id' => 'dago_hoodie', 'qty' => 1], ['item_id' => 'camera', 'qty' => 1]]], $tokenB);
check($r['status'] === 200 && qty($r['body']['inventory'], 'cucumber') === 3 && qty($r['body']['inventory'], 'dago_hoodie') === 0
    && count($r['body']['skipped']) === 2, 'import: cucumbers merged, physical/starter items skipped', $r);

// ── Temporary test codes (removed again at the end) ────────────────────────────────────────────────────
$tempCodes = [];
$tempCode = function (string $type, ?string $expiresAt = null, string $email = '') use (&$tempCodes): string {
    $code = 't' . substr(bin2hex(random_bytes(4)), 0, 7);
    Db::run("INSERT INTO qr (id, code, redirect_to, type, email, created_at, claimed_at, expires_at) VALUES (?, ?, 'https://scutoons.com/catpope', ?, ?, ?, ?, ?)",
        [Db::uuid(), $code, $type, $email, qrNow(), qrNow(), $expiresAt]);
    return $tempCodes[] = $code;
};
register_shutdown_function(function () use (&$tempCodes) {
    foreach ($tempCodes as $code) {
        Db::run('DELETE FROM inventory WHERE code = ?', [$code]);
        Db::run('DELETE FROM qr WHERE code = ?', [$code]);
    }
});
$userA = (string) Db::value('SELECT id FROM users WHERE email = ?', [$emailA]);

// ── /access/: device-level viewing for anyone holding a code ───────────────────────────────────────────
echo "\n/access/ (viewing is device-level)\n";
$unclaimed = $pick('3_catpope_vinyl');
$r = call('GET', "$api/access/?c=" . rawurlencode((string) $unclaimed));
check($r['status'] === 200 && $r['body']['permissions'] === ['catalog'] && $r['body']['claimable'] === true && $r['body']['claimed'] === false
    && !array_key_exists('claimed_by_me', $r['body']), 'access as guest, unclaimed code → catalog', $r);
$payload = Permissions::verifyToken((string) ($r['body']['access_token'] ?? ''));
check($payload !== null && $payload['perms'] === ['catalog'] && str_starts_with($payload['sub'], 'code:') && !str_contains($payload['sub'], (string) $unclaimed),
    'access_token verifies (sub = hashed code)', $payload);
$r = call('POST', "$api/access/", ['code' => $dago], $tokenA);
check($r['status'] === 200 && $r['body']['claimed'] === true && $r['body']['claimed_by_me'] === true && $r['body']['permissions'] === ['catalog'], 'access as owner → claimed_by_me', $r);
$r = call('POST', "$api/access/", ['code' => $dago], $tokenB);
check($r['status'] === 200 && $r['body']['claimed_by_me'] === false && $r['body']['permissions'] === ['catalog'] && isset($r['body']['access_token']),
    'access with a code owned by someone else → still catalog (viewing)', $r);
$r = call('POST', "$api/access/", ['code' => $dago]);
check($r['status'] === 200 && $r['body']['claimed'] === true && !preg_match('/@|claimed_by"/', (string) json_encode($r['body'])), 'access as guest, claimed code → catalog, owner never revealed', $r);
$r = call('GET', "$api/access/?c=" . $tempCode('membership', '2020-01-01 00:00:00'));
check($r['status'] === 410 && $r['body']['error'] === 'expired', 'access expired code → 410', $r);
$page = $tempCode('page');
$r = call('GET', "$api/access/?c=$page");
check($r['status'] === 200 && $r['body']['permissions'] === [] && $r['body']['claimable'] === false && !isset($r['body']['access_token']), 'access page code → no permissions, no token', $r);
$r = call('GET', "$api/access/?c=" . $tempCode('mystery'));
check($r['status'] === 200 && $r['body']['permissions'] === [] && $r['body']['claimable'] === false, 'access unknown type → default (nothing)', $r);
$r = call('GET', "$api/access/?c=zzzzzzzz");
check($r['status'] === 404, 'access unknown code → 404', $r['status']);
$r = call('POST', "$api/inventory/claim/", ['code' => $page], $tokenA);
check($r['status'] === 422 && $r['body']['error'] === 'not_claimable', 'claim page code → 422 not_claimable', $r);
$r = call('POST', "$api/inventory/claim/", ['code' => $tempCode('membership', null, 'legacy-owner@example.com')], $tokenA);
check($r['status'] === 409 && $r['body']['error'] === 'email_required', 'claim legacy email-claimed code with another account → 409 email_required', $r);

// ── /me/ access + signed tokens ────────────────────────────────────────────────────────────────────────
echo "\n/me/ access, access tokens\n";
$me = call('GET', "$api/me/", null, $tokenA);
$access = $me['body']['access'] ?? [];
check(($access['unlocked'] ?? false) === true && $access['permissions'] === ['catalog'] && in_array($vinyl, $access['codes'], true) && in_array($dago, $access['codes'], true)
    && $access['expires_at'] === null, '/me/ access from owned codes', $access);
$payload = Permissions::verifyToken((string) ($access['access_token'] ?? ''));
check($payload !== null && $payload['sub'] === "user:$userA" && $payload['exp'] <= time() + Permissions::TOKEN_MAX_SECONDS + 5, '/me/ access_token verifies (sub user, exp ≤ 30 days)', $payload);
$me = call('GET', "$api/me/", null, $tokenB);
check(($me['body']['access']['unlocked'] ?? null) === false && !isset($me['body']['access']['access_token']), '/me/ without owned codes → no access, no token', $me['body']['access'] ?? null);
[$p, $sig] = explode('.', (string) ($access['access_token'] ?? '.'));
$forged = json_decode((string) base64_decode(strtr($p, '-_', '+/')), true) ?: [];
$forged['perms'][] = 'admin';
$forgedPart = rtrim(strtr(base64_encode((string) json_encode($forged)), '+/', '-_'), '=');
check(Permissions::verifyToken("$forgedPart.$sig") === null, 'token with tampered perms fails');
check(Permissions::verifyToken(Permissions::issueToken(['catalog'], time() - 10, 'code:test')) === null, 'expired token fails');
check((Permissions::verifyToken(Permissions::issueToken(['catalog'], time() + 60, 'code:test'))['exp'] ?? PHP_INT_MAX) <= time() + 60, 'token exp = min(code expiry, 30 days)');
check(Permissions::verifyToken('garbage') === null && Permissions::verifyToken($p . '.' . strrev($sig)) === null, 'malformed / wrong signature fails');

// ── Permissions map is data, not code ──────────────────────────────────────────────────────────────────
echo "\nPermissions map\n";
$r = call('GET', "$api/permissions.json");
check(in_array($r['status'], [403, 404], true), 'api/permissions.json not served', $r['status']);
$r = call('GET', "$api/permissions/");
check($r['status'] === 404, 'no /api/permissions/ endpoint', $r['status']);
$override = tempnam(sys_get_temp_dir(), 'perm');
file_put_contents($override, json_encode(['version' => 1, 'permissions' => ['catalog' => '', 'toon:catpope' => ''],
    'types' => ['page' => ['claimable' => true, 'grants' => ['toon:catpope']]], 'default' => ['claimable' => false, 'grants' => []]]));
Permissions::useFile($override);
check(Permissions::claimable('page') && !Permissions::claimable('membership'), 'override map: page claimable, membership not');
$ownedNew = Codes::claim(Users::byId($userA), $page, null, null);
$access = Permissions::access(Db::all('SELECT code, type, expires_at FROM qr WHERE claimed_by = ?', [$userA]), $userA);
check($ownedNew && $access['permissions'] === ['toon:catpope'] && $access['unlocked'] === false, 'override map: page code claimed, grants toon:catpope only (data-driven)', $access);
Permissions::useFile(null);
unlink($override);
check(!Permissions::claimable('page') && Permissions::claimable('membership'), 'default map restored');

// ── Validate: backwards compatible for the old viewer ──────────────────────────────────────────────────
echo "\n/validate/ (legacy, backwards compatible)\n";
$r = call('POST', "$api/validate/", ['code' => $dago]);
$qr = $r['body']['qr'] ?? [];
check($r['status'] === 200 && $r['body']['status'] === 'success' && $qr['is_claimed'] === true, 'validate claimed code as guest → success', $r);
check(!array_key_exists('email', $qr) && !array_key_exists('signature', $qr) && !array_key_exists('claimed_by', $qr), 'validate has no email / signature / claimed_by', array_keys($qr));
$r = call('GET', "$api/validate/?c=" . rawurlencode((string) $unclaimed));
check($r['status'] === 200 && ($r['body']['qr']['is_claimed'] ?? null) === false, 'validate unclaimed code → success', $r);
$r = call('GET', "$api/validate/?c=zzzzzzzz");
check($r['status'] === 404 && $r['body']['status'] === 'error', 'validate unknown → 404', $r);

// ── Database guard: the triggers refuse a second owner even outside the API ────────────────────────────
echo "
Database owner guard
";
$userB = (string) Db::value('SELECT id FROM users WHERE email = ?', [$emailB]);
$refused = function (callable $write): bool {
    try { $write(); return false; } catch (Throwable) { return true; }
};
check($refused(fn() => Db::run('UPDATE qr SET claimed_by = ? WHERE code = ?', [$userB, $dago])), 'DB: re-assigning a claimed code to another account is refused');
check(Db::value('SELECT claimed_by FROM qr WHERE code = ?', [$dago]) !== $userB, 'DB: code still belongs to its first owner');
check($refused(fn() => Db::run("INSERT INTO inventory (id, user_id, item_id, variant, qty, source, code, edition_number, acquired_at, updated_at) VALUES (?, ?, 'dago_hoodie', 'black', 1, 'qr', ?, NULL, ?, ?)",
    [Db::uuid(), $userB, $dago, Db::now(), Db::now()])), 'DB: inventory row for a code owned by someone else is refused');

// ── Private key never served ───────────────────────────────────────────────────────────────────────────
$r = call('GET', "$api/keys/keys.php");
check(!is_array($r['body']) || !array_key_exists('privateKey', $r['body']), '/api/keys/keys.php has no private key');
$r = call('GET', "$api/keys/keypair.json");
check($r['status'] === 404 || $r['status'] === 403, '/api/keys/keypair.json not served', $r['status']);

// ── Logout ─────────────────────────────────────────────────────────────────────────────────────────────
echo "\nLogout\n";
$r = call('POST', "$api/auth/logout/", null, $tokenA);
check($r['status'] === 200 && $r['body']['status'] === 'success', 'auth/logout', $r);
$r = call('GET', "$api/me/", null, $tokenA);
check($r['status'] === 401, 'GET /me/ after logout → 401', $r['status']);

echo "\n" . ($checks - $failures) . " / $checks checks passed\n";
exit($failures === 0 ? 0 : 1);
