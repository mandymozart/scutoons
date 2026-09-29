<?php
/**
 * Admin: activity statistics per code (ITEMS.md "Revoke + activity statistics"). Admin token required.
 *   GET stats.php[?days=60]            → { success, codes: [...], daily: [{ day, scans, accesses }] }
 *   GET stats.php?format=csv           → the per-code table as CSV
 * Never returns emails or user ids.
 */
require_once __DIR__ . '/auth-check.php';
require_once __DIR__ . '/../api/lib/bootstrap.php';

$days = max(1, min(366, (int) ($_GET['days'] ?? 60)));

try {
    $agg = [];
    $rows = Db::all("SELECT code,
            SUM(CASE WHEN event = 'scan' THEN 1 ELSE 0 END) AS scans,
            SUM(CASE WHEN event = 'access' THEN 1 ELSE 0 END) AS accesses,
            COUNT(DISTINCT device_hash) AS devices,
            SUM(CASE WHEN event = 'login' THEN 1 ELSE 0 END) AS logins,
            SUM(CASE WHEN event = 'claim_refused' THEN 1 ELSE 0 END) AS refused,
            MIN(created_at) AS first_seen, MAX(created_at) AS last_seen
        FROM code_events GROUP BY code");
    foreach ($rows as $row) $agg[$row['code']] = $row;

    $codes = [];
    foreach (Db::all('SELECT code, type, edition_id, edition_number, created_at, claimed_at, claimed_by, email, expires_at, revoked_at, revoked_reason
                      FROM qr ORDER BY edition_id, edition_number, code') as $qr) {
        $a = $agg[$qr['code']] ?? [];
        $claimed = $qr['claimed_by'] !== null || (string) $qr['email'] !== '';
        $codes[] = [
            'code' => $qr['code'],
            'type' => $qr['type'],
            'edition_id' => $qr['edition_id'],
            'edition_number' => $qr['edition_number'] !== null ? (int) $qr['edition_number'] : null,
            'created_at' => $qr['created_at'],
            'claimed' => $claimed,
            'claimed_at' => $claimed ? $qr['claimed_at'] : null,
            'expires_at' => $qr['expires_at'],
            'revoked_at' => $qr['revoked_at'],
            'revoked_reason' => $qr['revoked_reason'],
            'scans' => (int) ($a['scans'] ?? 0),
            'accesses' => (int) ($a['accesses'] ?? 0),
            'devices' => (int) ($a['devices'] ?? 0),
            'logins' => (int) ($a['logins'] ?? 0),
            'refused' => (int) ($a['refused'] ?? 0),
            'first_seen' => Db::iso($a['first_seen'] ?? null),
            'last_seen' => Db::iso($a['last_seen'] ?? null),
        ];
    }

    if (($_GET['format'] ?? '') === 'csv') {
        header('Content-Type: text/csv; charset=utf-8');
        header('Content-Disposition: attachment; filename="scutoons-code-stats-' . gmdate('Y-m-d') . '.csv"');
        header('Cache-Control: no-store');
        $out = fopen('php://output', 'w');
        fputcsv($out, array_keys($codes[0] ?? ['code' => '']));
        foreach ($codes as $row) fputcsv($out, array_map(fn($v) => is_bool($v) ? ($v ? 'yes' : 'no') : $v, $row));
        exit;
    }

    $daily = array_map(fn($r) => ['day' => $r['day'], 'scans' => (int) $r['scans'], 'accesses' => (int) $r['accesses']],
        Db::all("SELECT SUBSTR(created_at, 1, 10) AS day,
                    SUM(CASE WHEN event = 'scan' THEN 1 ELSE 0 END) AS scans,
                    SUM(CASE WHEN event = 'access' THEN 1 ELSE 0 END) AS accesses
                 FROM code_events WHERE created_at >= ? GROUP BY SUBSTR(created_at, 1, 10) ORDER BY day",
            [gmdate('Y-m-d', time() - ($days - 1) * 86400) . ' 00:00:00']));

    header('Content-Type: application/json');
    header('Cache-Control: no-store');
    echo json_encode(['success' => true, 'days' => $days, 'codes' => $codes, 'daily' => $daily]);
} catch (Throwable $e) {
    error_log('[qr] stats: ' . $e->getMessage());
    http_response_code(500);
    header('Content-Type: application/json');
    echo json_encode(['success' => false, 'message' => 'Could not load statistics']);
}
