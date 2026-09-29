<?php
/**
 * Backfills qr.edition_number from the batch manifests (generate/files/batch/*.json: codes[].n).
 * Only codes listed in a manifest get a number; codes in no manifest stay NULL (see report-orphan-codes.php).
 * Existing non-NULL numbers are left alone unless --force.
 *
 *   php scripts/backfill-edition-numbers.php             update the configured (local) database
 *   php scripts/backfill-edition-numbers.php --dry-run   only print what would change
 *   php scripts/backfill-edition-numbers.php --sql       print UPDATE statements (for production: run in phpMyAdmin)
 *   --manifests=<dir>                                    another manifest folder (e.g. a copy from the server)
 */
require_once __DIR__ . '/lib-manifests.php';

$dryRun = in_array('--dry-run', $argv, true);
$sqlOnly = in_array('--sql', $argv, true);
$force = in_array('--force', $argv, true);
$manifests = readBatchManifests(manifestDirFromArgs($argv));

if ($sqlOnly) {
    // No database needed: statements for production (codes come from the manifests, never user input)
    echo "-- qr.edition_number backfill from " . count($manifests) . " manifest codes, generated " . gmdate('c') . "\n";
    foreach ($manifests as $code => $info) {
        $quoted = "'" . str_replace("'", "''", $code) . "'";
        $edition = "'" . str_replace("'", "''", $info['edition_id']) . "'";
        echo "UPDATE `qr` SET `edition_number` = {$info['n']} WHERE `code` = $quoted AND `edition_id` = $edition"
            . ($force ? '' : ' AND `edition_number` IS NULL') . ";\n";
    }
    exit(0);
}

require_once __DIR__ . '/../db.php';
$pdo = getDbConnection();

$select = $pdo->prepare('SELECT code, edition_id, edition_number FROM qr WHERE code = ?');
$update = $pdo->prepare('UPDATE qr SET edition_number = ? WHERE code = ?');
$stats = ['updated' => 0, 'unchanged' => 0, 'kept' => 0, 'missing_in_db' => 0, 'edition_mismatch' => 0];

$pdo->beginTransaction();
foreach ($manifests as $code => $info) {
    $select->execute([$code]);
    $row = $select->fetch(PDO::FETCH_ASSOC);
    if (!$row) {
        $stats['missing_in_db']++;
        echo "not in db: $code ({$info['file']})\n";
        continue;
    }
    if ((string) $row['edition_id'] !== $info['edition_id']) {
        $stats['edition_mismatch']++;
        echo "edition mismatch: $code db={$row['edition_id']} manifest={$info['edition_id']} – skipped\n";
        continue;
    }
    if ($row['edition_number'] !== null && !$force) {
        $stats[(int) $row['edition_number'] === $info['n'] ? 'unchanged' : 'kept']++;
        continue;
    }
    if (!$dryRun) $update->execute([$info['n'], $code]);
    $stats['updated']++;
}
$dryRun ? $pdo->rollBack() : $pdo->commit();

echo ($dryRun ? '[dry run] ' : '') . json_encode($stats) . "\n";
