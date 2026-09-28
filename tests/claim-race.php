<?php
/**
 * Race test: N separate PHP processes, each signed in as a different account, claim the SAME code at the same
 * moment. Exactly one must win; all others must get 409 already_claimed; the code and its inventory rows must end
 * up with that one owner.
 *
 *   php tests/claim-race.php [workers=8]
 *
 * Runs against the configured local database (config.local.php) – local copy only. Creates throwaway users.
 */
if (PHP_SAPI !== 'cli') {
    http_response_code(404);
    exit;
}

require_once __DIR__ . '/../api/lib/bootstrap.php';

// ── Worker: wait for the start signal, then claim ──────────────────────────────────────────────────────
if (($argv[1] ?? '') === '--worker') {
    [, , $userId, $code, $startAt] = $argv;
    while (microtime(true) < (float) $startAt) usleep(200);
    try {
        $new = Codes::claim(Users::byId($userId), $code, null, null);
        echo $new ? 'won' : 'already_yours';
    } catch (ApiError $e) {
        echo $e->errorCode;
    } catch (Throwable $e) {
        echo 'exception: ' . $e->getMessage();
    }
    exit;
}

// ── Parent ─────────────────────────────────────────────────────────────────────────────────────────────
$workers = max(2, (int) ($argv[1] ?? 8));
// A single-option code (vinyl), so every worker's claim is valid on its own
$code = Db::value("SELECT code FROM qr WHERE edition_id = '3_catpope_vinyl' AND claimed_by IS NULL AND email = '' AND (expires_at IS NULL OR expires_at > ?) ORDER BY edition_number LIMIT 1", [qrNow()]);
if (!$code) {
    fwrite(STDERR, "No unclaimed vinyl code left in the local DB.\n");
    exit(2);
}

$run = gmdate('YmdHis') . '-' . bin2hex(random_bytes(2));
$users = [];
for ($i = 0; $i < $workers; $i++) $users[] = Users::create("race-$run-$i@example.com", 'en');

$startAt = microtime(true) + 1.5; // time for every process to boot
$procs = [];
foreach ($users as $user) {
    $cmd = [PHP_BINARY, __FILE__, '--worker', $user['id'], $code, (string) $startAt];
    $procs[] = [$user, proc_open($cmd, [1 => ['pipe', 'w'], 2 => ['pipe', 'w']], $pipes), $pipes];
}

$results = [];
foreach ($procs as [$user, $proc, $pipes]) {
    $out = trim(stream_get_contents($pipes[1]) . stream_get_contents($pipes[2]));
    proc_close($proc);
    $results[$user['id']] = $out;
}

$winners = array_keys($results, 'won', true);
$losers = array_filter($results, fn($r) => $r === 'already_claimed');
$owner = Db::value('SELECT claimed_by FROM qr WHERE code = ?', [$code]);
$rowOwners = Db::all('SELECT DISTINCT user_id FROM inventory WHERE code = ?', [$code]);

echo "Code $code, $workers concurrent claims\n";
foreach (array_count_values($results) as $result => $count) echo "  $result: $count\n";

$ok = count($winners) === 1
    && count($losers) === $workers - 1
    && $owner === $winners[0]
    && count($rowOwners) === 1 && $rowOwners[0]['user_id'] === $winners[0];
echo $ok ? "PASS  exactly one owner; code and inventory agree\n" : "FAIL  " . json_encode(compact('winners', 'owner', 'rowOwners')) . "\n";
exit($ok ? 0 : 1);
