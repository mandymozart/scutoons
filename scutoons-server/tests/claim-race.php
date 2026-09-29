<?php
/**
 * Race tests with N separate PHP processes acting on the SAME free vinyl code at the same moment.
 *
 *   claim mode:  N signed-in accounts claim the code. Exactly one wins, all others get 409 already_claimed; the code
 *                and its inventory rows end up with that one owner.
 *   signup mode: N fresh emails confirm a login request carrying the code (Auth::confirm = /auth/verify/ after the
 *                OTP check). Exactly one account is created and it owns the code; all others get 409
 *                already_claimed and NO account (signup + claim are one transaction).
 *
 *   php tests/claim-race.php [workers=8] [claim|signup|both=both]
 *
 * Runs against the configured local database (DB_SQLITE_PATH / QR_DB_SQLITE_PATH) – local copy only. Creates throwaway users.
 */
if (PHP_SAPI !== 'cli') {
    http_response_code(404);
    exit;
}

require_once __DIR__ . '/support/legacy.php';

// ── Workers: wait for the start signal, then claim / confirm ───────────────────────────────────────────
if (($argv[1] ?? '') === '--worker' || ($argv[1] ?? '') === '--signup-worker') {
    [, $mode, $id, $code, $startAt] = $argv;
    while (microtime(true) < (float) $startAt) usleep(200);
    try {
        if ($mode === '--worker') {
            $new = Codes::claim(Users::byId($id), $code, null, null);
            echo $new ? 'won' : 'already_yours';
        } else {
            $request = Db::one('SELECT * FROM login_requests WHERE id = ?', [$id]);
            $result = Auth::confirm($request, []);
            echo ($result['created'] && ($result['claimed'] ?? []) === [$code]) ? 'won' : 'unexpected: ' . json_encode($result);
        }
    } catch (ApiError $e) {
        echo $e->errorCode;
    } catch (Throwable $e) {
        echo 'exception: ' . $e->getMessage();
    }
    exit;
}

// ── Parent ─────────────────────────────────────────────────────────────────────────────────────────────
$workers = max(2, (int) ($argv[1] ?? 8));
$modes = ($argv[2] ?? 'both') === 'both' ? ['claim', 'signup'] : [$argv[2]];

/** A free single-option code (vinyl), so every worker's claim is valid on its own */
function freeVinyl(): string
{
    $code = Db::value("SELECT code FROM qr WHERE edition_id = '3_catpope_vinyl' AND claimed_by IS NULL AND email = '' AND revoked_at IS NULL AND (expires_at IS NULL OR expires_at > ?) ORDER BY edition_number LIMIT 1", [qrNow()]);
    if (!$code) {
        fwrite(STDERR, "No unclaimed vinyl code left in the local DB.\n");
        exit(2);
    }
    return (string) $code;
}

/**
 * @param list<string> $ids
 * @return array<string, string> id → worker output
 */
function race(string $flag, array $ids, string $code): array
{
    $startAt = microtime(true) + 1.5; // time for every process to boot
    $procs = [];
    foreach ($ids as $id) {
        $cmd = [PHP_BINARY, __FILE__, $flag, $id, $code, (string) $startAt];
        $procs[] = [$id, proc_open($cmd, [1 => ['pipe', 'w'], 2 => ['pipe', 'w']], $pipes), $pipes];
    }
    $results = [];
    foreach ($procs as [$id, $proc, $pipes]) {
        $out = trim(stream_get_contents($pipes[1]) . stream_get_contents($pipes[2]));
        proc_close($proc);
        $results[$id] = $out;
    }
    return $results;
}

$run = gmdate('YmdHis') . '-' . bin2hex(random_bytes(2));
$allOk = true;

if (in_array('claim', $modes, true)) {
    $code = freeVinyl();
    $users = [];
    for ($i = 0; $i < $workers; $i++) $users[] = Users::create("race-$run-$i@example.com", 'en');
    $results = race('--worker', array_column($users, 'id'), $code);

    $winners = array_keys($results, 'won', true);
    $losers = array_filter($results, fn($r) => $r === 'already_claimed');
    $owner = Db::value('SELECT claimed_by FROM qr WHERE code = ?', [$code]);
    $rowOwners = Db::all('SELECT DISTINCT user_id FROM inventory WHERE code = ?', [$code]);

    echo "Claim: code $code, $workers concurrent claims\n";
    foreach (array_count_values($results) as $result => $count) echo "  $result: $count\n";
    $ok = count($winners) === 1
        && count($losers) === $workers - 1
        && $owner === $winners[0]
        && count($rowOwners) === 1 && $rowOwners[0]['user_id'] === $winners[0];
    echo $ok ? "PASS  exactly one owner; code and inventory agree\n" : "FAIL  " . json_encode(compact('winners', 'owner', 'rowOwners')) . "\n";
    $allOk = $allOk && $ok;
}

if (in_array('signup', $modes, true)) {
    $code = freeVinyl();
    $requestIds = []; // email → login request id
    for ($i = 0; $i < $workers; $i++) {
        $email = "race-signup-$run-$i@example.com";
        $requestIds[$email] = Db::uuid7();
        Db::run("INSERT INTO login_requests (id, email, token_hash, code_hash, locale, claim_code, ip_hash, attempts, created_at, expires_at) VALUES (?, ?, ?, ?, 'en', ?, 'race', 0, ?, ?)",
            [$requestIds[$email], $email, bin2hex(random_bytes(32)), str_repeat('0', 64), $code, Db::now(), Db::now(600)]);
    }
    $results = race('--signup-worker', array_values($requestIds), $code);

    $winners = array_keys($results, 'won', true);
    $losers = array_filter($results, fn($r) => $r === 'already_claimed');
    $emails = array_keys($requestIds);
    $accounts = Db::all('SELECT id, email FROM users WHERE email IN (' . implode(',', array_fill(0, count($emails), '?')) . ')', $emails);
    $owner = Db::value('SELECT claimed_by FROM qr WHERE code = ?', [$code]);
    $rowOwners = Db::all('SELECT DISTINCT user_id FROM inventory WHERE code = ?', [$code]);
    $winnerEmail = $winners ? array_search($winners[0], $requestIds, true) : null;

    echo "Signup: code $code, $workers fresh emails confirm at once\n";
    foreach (array_count_values($results) as $result => $count) echo "  $result: $count\n";
    $ok = count($winners) === 1
        && count($losers) === $workers - 1
        && count($accounts) === 1 && $accounts[0]['email'] === $winnerEmail
        && $owner === $accounts[0]['id']
        && count($rowOwners) === 1 && $rowOwners[0]['user_id'] === $accounts[0]['id'];
    echo $ok ? "PASS  exactly one account created, and it owns the code\n" : "FAIL  " . json_encode(compact('results', 'accounts', 'owner', 'rowOwners')) . "\n";
    $allOk = $allOk && $ok;
}

exit($allOk ? 0 : 1);
