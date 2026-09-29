<?php
/**
 * READ-ONLY report: codes with an edition_id that are in no batch manifest (generate/files/batch/*.json).
 * Background: the local DB has 74 dago codes (edition of 37) and 40 beetles codes (edition of 20) – earlier batch
 * runs inserted codes whose manifest was overwritten or never written. This script never changes or deletes
 * anything; decide per code what to do (they may already be printed on stickers).
 *
 *   php scripts/report-orphan-codes.php            summary + one line per orphan code
 *   php scripts/report-orphan-codes.php --csv      CSV (code, edition_id, signed edition number, created_at, claimed)
 *   --manifests=<dir>                              another manifest folder
 */
require_once __DIR__ . '/lib-manifests.php';
require_once __DIR__ . '/../db.php';

$csv = in_array('--csv', $argv, true);
$manifests = readBatchManifests(manifestDirFromArgs($argv));
$pdo = getDbConnection();

$rows = $pdo->query("SELECT code, edition_id, created_at, email, claimed_by, signature FROM qr
                     WHERE edition_id IS NOT NULL AND edition_id <> '' ORDER BY edition_id, created_at, code")->fetchAll(PDO::FETCH_ASSOC);

$editions = json_decode((string) file_get_contents(__DIR__ . '/../api/editions/editions.json'), true) ?: [];
$counts = [];
foreach ($editions as $edition) $counts[$edition['id']] = (int) ($edition['edition_count'] ?? 0);

$summary = [];
$orphans = [];
foreach ($rows as $row) {
    $edition = $row['edition_id'];
    $summary[$edition] ??= ['in_db' => 0, 'in_manifest' => 0, 'orphans' => 0, 'edition_count' => $counts[$edition] ?? null];
    $summary[$edition]['in_db']++;
    if (isset($manifests[$row['code']])) {
        $summary[$edition]['in_manifest']++;
        continue;
    }
    $summary[$edition]['orphans']++;
    // The signed payload carries the number printed on the certificate
    $signed = null;
    $signature = json_decode((string) $row['signature'], true);
    if (is_array($signature) && isset($signature['signedData'])) {
        $data = json_decode((string) $signature['signedData'], true);
        if (is_array($data) && isset($data['edition_number'])) $signed = (int) $data['edition_number'];
    }
    $orphans[] = [
        'code' => $row['code'],
        'edition_id' => $edition,
        'signed_number' => $signed,
        'created_at' => $row['created_at'],
        'claimed' => ($row['email'] !== '' && $row['email'] !== null) || $row['claimed_by'] !== null,
    ];
}

if ($csv) {
    $out = fopen('php://stdout', 'w');
    fputcsv($out, ['code', 'edition_id', 'signed_edition_number', 'created_at', 'claimed']);
    foreach ($orphans as $o) fputcsv($out, [$o['code'], $o['edition_id'], $o['signed_number'], $o['created_at'], $o['claimed'] ? 'yes' : 'no']);
    exit(0);
}

echo "Codes per edition (DB vs. batch manifests):\n";
foreach ($summary as $edition => $s) {
    printf("  %-20s db=%4d  in manifest=%4d  orphans=%4d  edition_count=%s\n",
        $edition, $s['in_db'], $s['in_manifest'], $s['orphans'], $s['edition_count'] ?? '?');
}
echo "\nOrphan codes (in no manifest) – nothing was changed:\n";
foreach ($orphans as $o) {
    printf("  %-8s  %-20s  signed #%-4s  created %s%s\n",
        $o['code'], $o['edition_id'], $o['signed_number'] ?? '?', $o['created_at'], $o['claimed'] ? '  CLAIMED' : '');
}
echo "\n" . count($orphans) . " orphan code(s).\n";
