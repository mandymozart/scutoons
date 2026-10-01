<?php
/**
 * End-to-end smoke test of the accounts/items API against a running local server.
 *
 *   composer serve                            (php -S localhost:9000 -t public public/router.php; MAIL_TRANSPORT = log)
 *   php tests/api-smoke.php [http://localhost:9000]
 *
 * Uses fresh email addresses per run and picks unclaimed codes from the local DB (DB_SQLITE_PATH, or a copy via QR_DB_SQLITE_PATH): it claims
 * one vinyl code (single option) and one Dago code (choice) – run it against a local copy only. Temporary codes
 * (promo, page, …) are created and removed again. Also calls bin/console roles.
 * OTPs are read from the mail log (logs/mail.log). Exit code 0 = all checks passed.
 */
if (PHP_SAPI !== 'cli') {
    http_response_code(404);
    exit;
}

require_once __DIR__ . '/support/legacy.php';

$base = rtrim($argv[1] ?? 'http://localhost:9000', '/');
// The QR landing page is the qr app (in front of this API, it logs the scans through /api/code/?scan=1)
$qrBase = rtrim($argv[2] ?? 'http://localhost:9001', '/');
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

/** The newest mail to $email (headers + body) from the mail log, or null */
function mailFor(string $email): ?string
{
    global $mailLog;
    clearstatcache();
    $log = (string) @file_get_contents($mailLog);
    $pos = strrpos($log, "To: $email\r\n");
    if ($pos === false) return null;
    $next = strpos($log, "\nTo: ", $pos + 5);
    return $next === false ? substr($log, $pos) : substr($log, $pos, $next - $pos);
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
        Db::run('DELETE FROM code_events WHERE code = ?', [$code]);
        Db::run('DELETE FROM qr WHERE code = ?', [$code]);
    }
});
$promo = $tempCode('promo');

// ── Public endpoints ───────────────────────────────────────────────────────────────────────────────────
echo "Public\n";
$r = call('GET', "$api/health/");
check($r['status'] === 200 && $r['body']['db'] === 'ok', 'GET /health/', $r);
$r = call('GET', "$api/items/");
check($r['status'] === 200 && count($r['body']['items'] ?? []) === 10 && ($r['body']['recipes'][0]['id'] ?? '') === 'cucumber_camera', 'GET /items/ → 10 items + recipes', $r['status']);
// the cucumber camera only scans cucumbers (owner 2026-09-30): crafting trades the starter camera's QR scan for it
$skillsOf = fn(string $id) => array_column(array_values(array_filter($r['body']['items'] ?? [], fn($i) => $i['id'] === $id))[0]['skills'] ?? [], 'type');
check($skillsOf('camera') === ['scan_qr'] && $skillsOf('cucumber_camera') === ['scan_object'], 'camera: scan_qr · cucumber_camera: scan_object only', [$skillsOf('camera'), $skillsOf('cucumber_camera')]);
// Shop data: the stored files byte-for-byte (config/products.json, config/featured-products.json = the default while
// nothing is chosen in the admin app – run-contract.php starts without stored settings), public + cached
foreach (['products' => 'config/products.json', 'products/featured' => 'config/featured-products.json'] as $path => $file) {
    $raw = @file_get_contents("$api/$path/", false, stream_context_create(['http' => ['header' => "Origin: https://evil.example\r\n", 'ignore_errors' => true]]));
    $head = strtolower(implode("\n", $http_response_header ?? []));
    check($raw !== false && $raw === file_get_contents(dirname(__DIR__) . "/$file") && str_contains($head, ' 200')
        && str_contains($head, 'access-control-allow-origin: *') && str_contains($head, 'cache-control: public, max-age=' . ($path === 'products' ? 300 : 60))
        && str_contains($head, 'content-type: application/json'), "GET /$path/ → $file byte-for-byte, CORS *, cached", $head);
}
$r = call('GET', "$api/products/", null, null, ['Origin: http://localhost:5173']);
check(($r['body']['version'] ?? null) === 2 && count($r['body']['products'] ?? []) > 0
    && ($r['headers']['access-control-allow-origin'] ?? '') === 'http://localhost:5173', 'GET /products/ → catalog v2, allowed origin echoed', $r['headers']);
$r = call('GET', "$api/products/featured/");
check(is_array($r['body']) && array_is_list($r['body']) && is_string($r['body'][0]['product'] ?? null), 'GET /products/featured/ → [{ product, variant }]', $r['body']);
$r = call('POST', "$api/products/", []);
check($r['status'] === 405 && ($r['body']['error'] ?? '') === 'method_not_allowed', 'POST /products/ → 405', $r);
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
check($v['status'] === 200 && $v['body']['created'] === true && $v['body']['claimed'] === [$vinyl], 'signup with a membership code → new account + vinyl claimed', $v);
check(($v['body']['user']['roles'] ?? null) === ['member'] && ($v['body']['user']['marketing_opt_in'] ?? null) === false, 'new account: roles [member], opt-in false by default', $v['body']['user'] ?? null);
check(($v['body']['user']['hide_sensitive'] ?? null) === true, 'new account: hide_sensitive true by default', $v['body']['user'] ?? null);
check(Db::value('SELECT signup_code FROM users WHERE email = ?', [$emailA]) === $vinyl, 'users.signup_code = the membership code');
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
echo "\nUser B (signs up with a promo code)\n";
$b = login($emailB, ['code' => $promo]);
check(($b['created'] ?? null) === true && !array_key_exists('claimed', $b) && !array_key_exists('claim_error', $b), 'promo signup: account created, nothing claimed', $b);
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

$userA = (string) Db::value('SELECT id FROM users WHERE email = ?', [$emailA]);

// ── Signup rule: new accounts only with a product or promo code ────────────────────────────────────────
echo "\nSignup rule\n";
$emailC = "smoke-$run-c@example.com";
$emailD = "smoke-$run-d@example.com";
$noAccount = fn(string $email) => Db::value('SELECT 1 FROM users WHERE email = ?', [$email]) === null;
$r = call('POST', "$api/auth/request/", ['email' => $emailC, 'locale' => 'en']);
check($r['status'] === 200 && ($r['body']['status'] ?? null) === 'sent', 'no account, no code → still {status:"sent"}', $r);
$mail = (string) mailFor($emailC);
check(str_contains($mail, 'There is no account for it yet') && str_contains($mail, 'promo code') && otpFor($emailC) === null && !str_contains($mail, '?login='),
    'no account, no code → mail explains the rule, no one-time code, no link', $mail);
$r = call('POST', "$api/auth/verify/", ['email' => $emailC, 'otp' => '123456']);
check($r['status'] === 400 && $noAccount($emailC), 'no account, no code → verify fails, no account created', $r);
call('POST', "$api/auth/request/", ['email' => $emailC, 'locale' => 'de', 'code' => $tempCode('page')]);
$mail = (string) mailFor($emailC);
check(str_contains($mail, 'noch kein Konto') && str_contains($mail, 'kann kein Konto erstellt werden') && otpFor($emailC) === null, 'page code → no signup (German mail names the code)', $mail);
call('POST', "$api/auth/request/", ['email' => $emailC, 'locale' => 'en', 'code' => $dago]);
check(otpFor($emailC) === null && $noAccount($emailC), 'membership code owned by someone else → no signup');
$revokedPromo = $tempCode('promo');
Db::run('UPDATE qr SET revoked_at = ? WHERE code = ?', [qrNow(), $revokedPromo]);
call('POST', "$api/auth/request/", ['email' => $emailC, 'locale' => 'en', 'code' => $revokedPromo]);
check(otpFor($emailC) === null && $noAccount($emailC), 'revoked promo code → no signup');

$lateRevoke = $tempCode('promo');
call('POST', "$api/auth/request/", ['email' => $emailC, 'locale' => 'en', 'code' => $lateRevoke, 'marketing_opt_in' => true]);
$otpC = otpFor($emailC);
check($otpC !== null && str_contains((string) mailFor($emailC), 'This creates your SCUTOONS account'), 'promo code → sign-up mail with one-time code');
Db::run('UPDATE qr SET revoked_at = ? WHERE code = ?', [qrNow(), $lateRevoke]);
$r = call('POST', "$api/auth/verify/", ['email' => $emailC, 'otp' => $otpC]);
check($r['status'] === 403 && $r['body']['error'] === 'signup_code_required' && $noAccount($emailC), 'code revoked before verify → 403 signup_code_required, no account', $r);

$c = login($emailC, ['code' => $promo, 'marketing_opt_in' => true]);
$tokenC = $c['session_token'];
check($c['created'] === true && $c['user']['marketing_opt_in'] === true && $c['user']['roles'] === ['member'] && !array_key_exists('claimed', $c),
    'second signup with the same promo code → account, opt-in stored', $c);
$d = login($emailD, ['code' => $promo], ['marketing_opt_in' => false]);
check($d['created'] === true && $d['user']['marketing_opt_in'] === false, 'third signup with the same promo code (reusable), opt-in false', $d);
$promoRow = Codes::find($promo);
check($promoRow['claimed_by'] === null && (string) $promoRow['email'] === '', 'promo code is not owned by anyone');
check(Db::value('SELECT signup_code FROM users WHERE email = ?', [$emailD]) === $promo, 'users.signup_code = the promo code');
check((int) Db::value("SELECT COUNT(*) FROM code_events WHERE code = ? AND event = 'signup'", [$promo]) === 3, 'signup events logged (3 for the promo code)');
check(Db::value('SELECT marketing_opt_in_at FROM users WHERE email = ?', [$emailC]) !== null, 'marketing_opt_in_at set when opted in');
$r = call('POST', "$api/inventory/claim/", ['code' => $promo], $tokenC);
check($r['status'] === 422 && $r['body']['error'] === 'not_claimable', 'claim promo code → 422 not_claimable', $r);
$r = call('GET', "$api/access/?c=$promo");
check($r['status'] === 200 && $r['body']['permissions'] === ['catalog'] && $r['body']['claimable'] === false && $r['body']['signup'] === true, '/access/ promo → catalog, claimable false, signup true', $r);
$r = call('GET', "$api/code/?c=$promo");
check($r['status'] === 200 && $r['body']['claimable'] === false && $r['body']['signup'] === true, '/code/ promo → claimable false, signup true', $r);
$r = call('GET', "$qrBase/?c=$promo&lang=en");
check($r['status'] === 200 && is_string($r['body']) && str_contains($r['body'], 'no item to collect') && str_contains($r['body'], '/account?code=' . $promo)
    && str_contains($r['body'], 'Create account') && !str_contains($r['body'], 'Add to my account'), 'landing page promo: no item, Continue + Create account', $r['status']);
$r = call('GET', "$qrBase/?c=$vinyl&lang=en");
check(is_string($r['body']) && str_contains($r['body'], 'Add to my account'), 'landing page membership: still Add to my account');

$a2 = login($emailA);
check($a2['created'] === false && ($a2['user']['email'] ?? null) === $emailA, 'existing account without a code → sign-in works');
$a3 = login($emailA, ['code' => $tempCode('page')]);
check($a3['created'] === false && !array_key_exists('claim_error', $a3), 'existing account with a page code → sign-in works, no claim attempted', $a3);
check(Db::value('SELECT signup_code FROM users WHERE email = ?', [$emailA]) === $vinyl, 'signup_code unchanged by later sign-ins');
$a4 = login($emailA, null, ['marketing_opt_in' => true]);
check($a4['created'] === false && $a4['user']['marketing_opt_in'] === true, 'existing account ticking the opt-in at sign-in → opted in', $a4['user'] ?? $a4);
$a5 = login($emailA, null, ['marketing_opt_in' => false]);
check($a5['user']['marketing_opt_in'] === true, 'signing in without the tick never opts out', $a5['user'] ?? $a5);

// ── Promo accounts read on every device: /me access from the signup code ─────────────────────────────
echo "\nPromo account access (/me)\n";
$me = call('GET', "$api/me/", null, $tokenC);
$access = $me['body']['access'] ?? [];
check(($access['unlocked'] ?? false) === true && $access['permissions'] === ['catalog'] && $access['signup_code'] === $promo && $access['codes'] === [$promo]
    && isset($access['access_token']) && $me['body']['memberships'] === [], '/me/ access via the promo signup code (not owned, no membership)', $me['body']);
check(Codes::find($promo)['claimed_by'] === null, 'promo code still owned by nobody');
$me = call('GET', "$api/me/", null, $tokenA);
check(array_key_exists('signup_code', $me['body']['access'] ?? []) && $me['body']['access']['signup_code'] === null && in_array($vinyl, $me['body']['access']['codes'], true), '/me/ membership signup code counts as owned code (signup_code null)', $me['body']['access'] ?? null);

// ── Signup + claim are atomic ─────────────────────────────────────────────────────────────────────────
echo "\nAtomic signup + claim\n";
$choice = $tempCode('membership');
Db::run("UPDATE qr SET edition_id = '1_catpope_dago' WHERE code = ?", [$choice]);
$emailF = "smoke-$run-f@example.com";
call('POST', "$api/auth/request/", ['email' => $emailF, 'locale' => 'en', 'code' => $choice]);
$otpF = otpFor($emailF);
$r = call('POST', "$api/auth/verify/", ['email' => $emailF, 'otp' => $otpF]);
check($r['status'] === 422 && $r['body']['error'] === 'choose_item' && count($r['body']['options'] ?? []) === 2 && $noAccount($emailF),
    'signup with a code that needs a choice, no choice → 422 choose_item, no account', $r);
$r = call('POST', "$api/auth/verify/", ['email' => $emailF, 'otp' => $otpF, 'item_id' => 'dago_heavy_t', 'variant' => 'white']);
check($r['status'] === 200 && $r['body']['created'] === true && $r['body']['claimed'] === [$choice] && Codes::find($choice)['claimed_by'] === $r['body']['user']['id'],
    'same one-time code again with the choice → account + claim', $r);
$taken = $tempCode('membership');
$emailG = "smoke-$run-g@example.com";
call('POST', "$api/auth/request/", ['email' => $emailG, 'locale' => 'en', 'code' => $taken]);
$otpG = otpFor($emailG);
$r = call('POST', "$api/inventory/claim/", ['code' => $taken], $tokenC);
check($r['status'] === 200, 'someone else claims the code between request and verify', $r);
$r = call('POST', "$api/auth/verify/", ['email' => $emailG, 'otp' => $otpG]);
check($r['status'] === 409 && $r['body']['error'] === 'already_claimed' && $noAccount($emailG), 'verify then → 409 already_claimed, no account', $r);
$signupRequest = [
    'id' => 'x', 'email' => "smoke-$run-h@example.com", 'locale' => 'en', 'claim_code' => $choice,
    'claim_item_id' => null, 'claim_variant' => null, 'marketing_opt_in' => 0,
];
Db::run("INSERT INTO login_requests (id, email, token_hash, code_hash, locale, claim_code, ip_hash, attempts, created_at, expires_at) VALUES (?, ?, ?, ?, 'en', ?, 'x', 0, ?, ?)",
    [$signupRequest['id'] = Db::uuid7(), $signupRequest['email'], bin2hex(random_bytes(32)), str_repeat('0', 64), $choice, Db::now(), Db::now(600)]);
try { Auth::confirm($signupRequest, []); $refusedInner = false; } catch (ApiError $err) { $refusedInner = $err->errorCode === 'already_claimed'; }
check($refusedInner && $noAccount($signupRequest['email']) && Db::value('SELECT used_at FROM login_requests WHERE id = ?', [$signupRequest['id']]) === null,
    'Auth::confirm with an owned product code → already_claimed, rolled back (no user, request unused)');

// ── Roles ──────────────────────────────────────────────────────────────────────────────────────────────
echo "\nRoles\n";
$me = call('GET', "$api/me/", null, $tokenC);
check(($me['body']['user']['roles'] ?? null) === ['member'], '/me/ user.roles = [member]', $me['body']['user'] ?? $me);
$userC = (string) Db::value('SELECT id FROM users WHERE email = ?', [$emailC]);
$adminHeader = ['X-Generate-Token: ' . hash_hmac('sha256', 'generate-access', GENERATE_PASSWORD)];
$r = call('GET', "$base/generate/users.php?q=" . rawurlencode($emailC));
check($r['status'] === 401, 'admin users.php without token → 401', $r['status']);
$r = call('POST', "$base/generate/user-roles.php", ['user_id' => $userC, 'role' => 'creator', 'action' => 'grant']);
check($r['status'] === 401 && !Roles::has($userC, 'creator'), 'admin user-roles.php without token → 401, nothing granted', $r['status']);
$r = call('GET', "$base/generate/users.php?q=" . rawurlencode("smoke-$run-c@"), null, null, $adminHeader);
$row = $r['body']['users'][0] ?? null;
check($r['status'] === 200 && $r['body']['total'] === 1 && $row['email'] === $emailC && $row['roles'] === ['member'] && $row['signup_code'] === $promo
    && $row['marketing_opt_in'] === true && in_array('creator', array_column($r['body']['roles'], 'name'), true), 'admin users.php search by email → roles, signup code, opt-in', $r['body']);
$_SERVER['HTTP_AUTHORIZATION'] = "Bearer $tokenC";
try { Roles::requireRole('creator'); $forbidden = false; } catch (ApiError $e) { $forbidden = $e->status === 403 && $e->errorCode === 'forbidden'; }
check($forbidden, 'Roles::requireRole(creator) without the role → 403 forbidden');
$r = call('POST', "$base/generate/user-roles.php", ['user_id' => $userC, 'role' => 'creator', 'action' => 'grant'], null, $adminHeader);
check($r['status'] === 200 && $r['body']['changed'] === true && $r['body']['user']['roles'] === ['member', 'creator'], 'admin grants creator', $r['body']);
$me = call('GET', "$api/me/", null, $tokenC);
check(($me['body']['user']['roles'] ?? null) === ['member', 'creator'] && Roles::has($userC, 'creator'), '/me/ user.roles includes creator', $me['body']['user'] ?? null);
check((Roles::requireRole('creator')['id'] ?? null) === $userC, 'Roles::requireRole(creator) with the role → the user');
unset($_SERVER['HTTP_AUTHORIZATION']);
$r = call('POST', "$base/generate/user-roles.php", ['user_id' => $userC, 'role' => 'creator', 'action' => 'grant'], null, $adminHeader);
check($r['status'] === 200 && $r['body']['changed'] === false, 'grant again → unchanged', $r['body']);
$r = call('POST', "$base/generate/user-roles.php", ['email' => $emailC, 'role' => 'creator', 'action' => 'revoke'], null, $adminHeader);
check($r['status'] === 200 && $r['body']['changed'] === true && $r['body']['user']['roles'] === ['member'] && !Roles::has($userC, 'creator'), 'admin revokes creator (by email)', $r['body']);
$r = call('POST', "$base/generate/user-roles.php", ['user_id' => $userC, 'role' => 'wizard', 'action' => 'grant'], null, $adminHeader);
check($r['status'] === 400, 'unknown role → 400', $r);
$cli = fn(string $args) => (string) shell_exec(escapeshellarg(PHP_BINARY) . ' ' . escapeshellarg(__DIR__ . '/../bin/console') . " roles $args 2>&1");
$out = $cli('grant ' . escapeshellarg($emailC) . ' admin');
check(str_contains($out, 'Granted admin') && Roles::has($userC, 'admin'), 'CLI roles.php grant', $out);
$out = $cli('list ' . escapeshellarg($emailC));
check(str_contains($out, 'member, admin'), 'CLI roles.php list <email>', $out);
$out = $cli('revoke ' . escapeshellarg($emailC) . ' admin');
check(str_contains($out, 'Revoked admin') && !Roles::has($userC, 'admin'), 'CLI roles.php revoke', $out);

// ── Marketing opt-in on the account page ──────────────────────────────────────────────────────────────
echo "\nMarketing opt-in\n";
$r = call('PUT', "$api/me/", ['marketing_opt_in' => false]);
check($r['status'] === 401, 'PUT /me/ without token → 401', $r['status']);
$r = call('PUT', "$api/me/", ['marketing_opt_in' => 'maybe'], $tokenC);
check($r['status'] === 400 && $r['body']['error'] === 'invalid_body', 'PUT /me/ with a non-boolean → 400', $r);
$r = call('PUT', "$api/me/", ['marketing_opt_in' => false], $tokenC);
check($r['status'] === 200 && $r['body']['user']['marketing_opt_in'] === false && $r['body']['user']['hide_sensitive'] === true, 'PUT /me/ opt-out (hide_sensitive untouched)', $r);
$r = call('PUT', "$api/me/", ['marketing_opt_in' => true], $tokenB);
$me = call('GET', "$api/me/", null, $tokenB);
check($r['status'] === 200 && ($me['body']['user']['marketing_opt_in'] ?? null) === true && Db::value('SELECT marketing_opt_in_at FROM users WHERE email = ?', [$emailB]) !== null,
    'PUT /me/ opt-in → /me/ shows it, time stored', $me['body']['user'] ?? $r);

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
$emailE = "smoke-$run-e@example.com";
$promoE = $tempCode('promo');
$e = login($emailE, ['code' => $promoE]);
Db::run('UPDATE qr SET revoked_at = ? WHERE code = ?', [qrNow(), $promoE]);
$me = call('GET', "$api/me/", null, $e['session_token']);
check(($me['body']['access']['unlocked'] ?? null) === false && !isset($me['body']['access']['access_token']) && array_key_exists('signup_code', $me['body']['access'] ?? []) && $me['body']['access']['signup_code'] === null,
    '/me/ with no owned code and a revoked signup code → no access, no token', $me['body']['access'] ?? null);
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

// ── Revoke (admin) ─────────────────────────────────────────────────────────────────────────────────────
echo "\nRevoke + admin endpoints\n";
$admin = ['X-Generate-Token: ' . hash_hmac('sha256', 'generate-access', GENERATE_PASSWORD)];
foreach (['stats.php', 'code-events.php?code=' . $dago] as $path) {
    $r = call('GET', "$base/generate/$path");
    check($r['status'] === 401, "admin $path without token → 401", $r['status']);
}
$r = call('POST', "$base/generate/revoke.php", ['code' => $dago, 'action' => 'revoke']);
check($r['status'] === 401, 'admin revoke.php without token → 401', $r['status']);
$rv = $tempCode('membership');
$r = call('POST', "$api/inventory/claim/", ['code' => $rv], $tokenA);
check($r['status'] === 200 && $r['body']['claimed'] === 'new', 'claim a temp code (to be revoked)', $r);
$r = call('POST', "$base/generate/revoke.php", ['code' => $rv, 'action' => 'revoke', 'reason' => 'smoke test'], null, $admin);
check($r['status'] === 200 && $r['body']['success'] === true && $r['body']['revoked_reason'] === 'smoke test', 'admin revokes the code', $r);
$r = call('GET', "$api/access/?c=$rv");
check($r['status'] === 410 && $r['body']['error'] === 'revoked', 'revoked: /access/ → 410 revoked', $r);
$r = call('POST', "$api/validate/", ['code' => $rv]);
check($r['status'] === 410 && $r['body']['error'] === 'revoked', 'revoked: /validate/ → 410 revoked', $r);
$r = call('POST', "$api/inventory/claim/", ['code' => $rv], $tokenB);
check($r['status'] === 410 && $r['body']['error'] === 'revoked', 'revoked: claim → 410 revoked', $r);
$me = call('GET', "$api/me/", null, $tokenA);
check(!in_array($rv, $me['body']['access']['codes'] ?? [], true) && in_array($vinyl, $me['body']['access']['codes'] ?? [], true), 'revoked: /me access ignores it', $me['body']['access'] ?? null);
$r = call('GET', "$qrBase/?c=$rv");
check($r['status'] === 410 && is_string($r['body']) && str_contains($r['body'], 'no longer valid'), 'revoked: landing page says no longer valid', $r['status']);
$r = call('POST', "$base/generate/revoke.php", ['code' => $rv, 'action' => 'unrevoke'], null, $admin);
check($r['status'] === 200 && $r['body']['revoked_at'] === null, 'admin unrevokes the code', $r);
$r = call('GET', "$api/access/?c=$rv");
check($r['status'] === 200 && $r['body']['permissions'] === ['catalog'], 'unrevoked: /access/ works again', $r);
$me = call('GET', "$api/me/", null, $tokenA);
check(in_array($rv, $me['body']['access']['codes'] ?? [], true), 'unrevoked: back in /me access');

// ── Activity events ────────────────────────────────────────────────────────────────────────────────────
echo "\nActivity events\n";
$ev = $tempCode('membership');
$count = fn(string $event) => (int) Db::value('SELECT COUNT(*) FROM code_events WHERE code = ? AND event = ?', [$ev, $event]);
$dev1 = ['X-Device-Id: smoke-' . $run . '-device-1'];
$dev2 = ['X-Device-Id: smoke-' . $run . '-device-2'];
call('GET', "$api/access/?c=$ev", null, null, $dev1);
call('GET', "$api/access/?c=$ev&toon=catpope", null, null, $dev1);
check($count('access') === 1, 'access: same device twice a day → 1 event', $count('access'));
call('POST', "$api/access/", ['code' => $ev], null, $dev2);
check($count('access') === 2, 'access: second device → 2 events', $count('access'));
check((int) Db::value('SELECT COUNT(DISTINCT device_hash) FROM code_events WHERE code = ?', [$ev]) === 2, 'events store 2 distinct device hashes');
check(Db::value("SELECT 1 FROM code_events WHERE code = ? AND (device_hash LIKE 'smoke-%' OR ip_hash LIKE '127.%')", [$ev]) === null, 'no raw device id or IP stored');
$r = call('OPTIONS', "$api/access/", null, null, ['Origin: http://localhost:5173', 'Access-Control-Request-Headers: x-device-id']);
check(str_contains(strtolower($r['headers']['access-control-allow-headers'] ?? ''), 'x-device-id'), 'CORS allows X-Device-Id', $r['headers']);
call('GET', "$qrBase/?c=$ev");
check($count('scan') === 1, 'scan: landing page visit → 1 event', $count('scan'));
call('POST', "$api/inventory/claim/", ['code' => $ev], $tokenA);
check($count('claim') === 1, 'claim → 1 event', $count('claim'));
call('POST', "$api/inventory/claim/", ['code' => $ev], $tokenB);
check($count('claim_refused') === 1, 'claim by another account → claim_refused', $count('claim_refused'));
$loginCount = (int) Db::value("SELECT COUNT(*) FROM code_events WHERE code = ? AND event = 'login' AND user_id = ?", [$vinyl, $userA]);
check($loginCount >= 1, 'login at verify → login event for the owned vinyl code', $loginCount);
$r = call('GET', "$base/generate/stats.php", null, null, $admin);
$row = array_values(array_filter($r['body']['codes'] ?? [], fn($c) => $c['code'] === $ev))[0] ?? null;
check($row && $row['accesses'] === 2 && $row['devices'] === 2 && $row['scans'] === 1 && $row['refused'] === 1 && $row['claimed'] === true
    && !preg_match('/@/', (string) json_encode($r['body'])), 'admin stats.php aggregates the code, no emails', $row);
$r = call('GET', "$base/generate/code-events.php?code=$ev", null, null, $admin);
check($r['status'] === 200 && count($r['body']['events'] ?? []) === 5, 'admin code-events.php timeline (5 events)', $r['body']['events'] ?? $r);
$ch = curl_init("$base/generate/stats.php?format=csv");
curl_setopt_array($ch, [CURLOPT_RETURNTRANSFER => true, CURLOPT_HTTPHEADER => $admin]);
$csv = (string) curl_exec($ch);
curl_close($ch);
check(str_starts_with($csv, 'code,type,edition_id') && str_contains($csv, $ev), 'admin CSV export');

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

// ── Single sign-on: the session cookie (docs/API.md "Single sign-on") ────────────────────────────────────
echo "\nSingle sign-on (session cookie)\n";
$app = 'http://localhost:8000';   // an ALLOWED_ORIGINS app (default and local config)
$cookieOf = fn(array $r): ?string => preg_match('/^scutoons_sid=([A-Za-z0-9_-]*);/', $r['headers']['set-cookie'] ?? '', $m) ? $m[1] : null;
$r = call('POST', "$api/auth/request/", ['email' => $emailA, 'locale' => 'en'], null, ["Origin: $app"]);
check($r['status'] === 200, 'SSO: auth/request from an app', $r);
$v = call('POST', "$api/auth/verify/", ['email' => $emailA, 'otp' => otpFor($emailA)], null, ["Origin: $app"]);
$sso = $cookieOf($v);
$setCookie = $v['headers']['set-cookie'] ?? '';
check($v['status'] === 200 && is_string($v['body']['session_token'] ?? null) && $sso === $v['body']['session_token'],
    'SSO: verify sets the cookie (same token; the body keeps session_token for old clients)', $v['headers']);
check(str_contains($setCookie, 'HttpOnly') && str_contains($setCookie, 'SameSite=Lax') && str_contains($setCookie, 'Path=/api') && str_contains($setCookie, 'Max-Age='),
    'SSO: cookie is HttpOnly, SameSite=Lax, Path=/api, with a lifetime', $setCookie);
check(($v['headers']['access-control-allow-credentials'] ?? '') === 'true', 'SSO: CORS allows credentials for the app', $v['headers']);
$v2 = call('POST', "$api/auth/verify/", ['email' => 'nobody@example.com', 'otp' => '000000'], null, ["Origin: $app"]);
check(!isset($v2['headers']['set-cookie']), 'SSO: no cookie for a failed verify', $v2['headers']);
$v3 = call('POST', "$api/auth/request/", ['email' => $emailB, 'locale' => 'en']);
$v3 = call('POST', "$api/auth/verify/", ['email' => $emailB, 'otp' => otpFor($emailB)]);
check($v3['status'] === 200 && !isset($v3['headers']['set-cookie']), 'SSO: verify without an app Origin (old clients, other sites) sets no cookie', $v3['headers']);
$ck = ["Cookie: scutoons_sid=$sso"];
$r = call('GET', "$api/me/", null, null, array_merge($ck, ["Origin: $app"]));
check($r['status'] === 200 && ($r['body']['user']['email'] ?? null) === $emailA, 'SSO: GET /me/ with the cookie only', $r);
$r = call('GET', "$api/me/", null, $tokenA);
check($r['status'] === 200 && ($r['body']['user']['email'] ?? null) === $emailA, 'SSO: the Bearer token still works', $r);
$r = call('GET', "$api/me/", null, $tokenB, $ck);
check($r['status'] === 200 && ($r['body']['user']['email'] ?? null) === $emailB, 'SSO: Bearer wins over the cookie', $r);
$r = call('GET', "$api/auth/session/", null, null, array_merge($ck, ["Origin: $app"]));
check($r['status'] === 200 && $r['body']['via'] === 'cookie' && $r['body']['user']['email'] === $emailA && $cookieOf($r) === $sso,
    'SSO: GET /auth/session/ → user, via cookie, cookie renewed', $r);
$r = call('GET', "$api/auth/session/");
check($r['status'] === 200 && $r['body']['user'] === null, 'SSO: GET /auth/session/ signed out → user null', $r);
$r = call('PUT', "$api/me/", ['marketing_opt_in' => false], null, array_merge($ck, ['Origin: https://evil.example']));
check($r['status'] === 403 && ($r['body']['error'] ?? null) === 'csrf_rejected', 'SSO: CSRF – cookie + foreign Origin on PUT → 403 csrf_rejected', $r);
$r = call('PUT', "$api/me/", ['marketing_opt_in' => false], null, $ck);
check($r['status'] === 403 && ($r['body']['error'] ?? null) === 'csrf_rejected', 'SSO: CSRF – cookie without Origin on PUT → 403', $r);
$r = call('POST', "$api/auth/logout/", null, null, array_merge($ck, ['Origin: https://evil.example']));
check($r['status'] === 403, 'SSO: CSRF – logout from a foreign Origin refused', $r);
$r = call('PUT', "$api/me/", ['marketing_opt_in' => false], null, array_merge($ck, ["Origin: $app"]));
check($r['status'] === 200, 'SSO: cookie + app Origin on PUT → 200', $r);
$r = call('GET', "$api/admin/roles", null, null, array_merge($ck, ["Origin: $app"]));
check($r['status'] === 403 && ($r['body']['role'] ?? null) === 'admin', 'SSO: role checks unchanged (member cookie → 403 admin)', $r);
$r = call('GET', "$api/validate/?c=$vinyl", null, null, array_merge($ck, ['Origin: https://qr.scutoons.com']));
check($r['status'] === 200, 'SSO: printed /validate/ unaffected by the cookie', $r['status']);
// exchange: an app's stored token → the same session in the cookie
$r = call('POST', "$api/auth/session/", null, $tokenA, ["Origin: $app"]);
check($r['status'] === 200 && $cookieOf($r) === $tokenA && $r['body']['via'] === 'cookie', 'SSO: POST /auth/session/ moves a stored Bearer token into the cookie', $r);
$r = call('POST', "$api/auth/session/", null, null, array_merge($ck, ["Origin: $app"]));
check($r['status'] === 400 && $r['body']['error'] === 'bearer_required', 'SSO: the exchange needs the Bearer token', $r);
// logout with the cookie ends the shared session (every app) and clears the cookie
$r = call('POST', "$api/auth/logout/", null, null, array_merge($ck, ["Origin: $app"]));
check($r['status'] === 200 && $cookieOf($r) === '' && str_contains($r['headers']['set-cookie'] ?? '', 'Max-Age=0'), 'SSO: logout clears the cookie', $r['headers']);
$r = call('GET', "$api/me/", null, null, array_merge($ck, ['Origin: http://localhost:11000']));
check($r['status'] === 401, 'SSO: after logout the cookie works nowhere (→ 401 in every app)', $r['status']);
$r = call('GET', "$api/auth/session/", null, null, array_merge($ck, ["Origin: $app"]));
check($r['status'] === 200 && $r['body']['user'] === null, 'SSO: after logout GET /auth/session/ → user null', $r);
$r = call('GET', "$api/me/", null, $tokenA);
check($r['status'] === 200, 'SSO: other sessions of the account (other devices) stay signed in', $r['status']);

// ── Logout ─────────────────────────────────────────────────────────────────────────────────────────────
echo "\nLogout\n";
$r = call('POST', "$api/auth/logout/", null, $tokenA);
check($r['status'] === 200 && $r['body']['status'] === 'success', 'auth/logout', $r);
$r = call('GET', "$api/me/", null, $tokenA);
check($r['status'] === 401, 'GET /me/ after logout → 401', $r['status']);

echo "\n" . ($checks - $failures) . " / $checks checks passed\n";
exit($failures === 0 ? 0 : 1);
