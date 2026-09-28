<?php
/**
 * Shared by the CLI scripts: reads the batch manifests in generate/files/batch/*.json
 * (written by the generate SPA's batch mode) into code => [edition_id, n, file].
 */
if (PHP_SAPI !== 'cli') {
    http_response_code(404);
    exit;
}

/** @return array<string, array{edition_id: string, n: int, file: string, status: string}> */
function readBatchManifests(string $dir): array
{
    $codes = [];
    foreach (glob(rtrim($dir, '/\\') . '/BATCH-*.json') ?: [] as $file) {
        $manifest = json_decode((string) file_get_contents($file), true);
        if (!is_array($manifest) || !isset($manifest['codes']) || !is_array($manifest['codes'])) {
            fwrite(STDERR, "skipping invalid manifest " . basename($file) . "\n");
            continue;
        }
        foreach ($manifest['codes'] as $entry) {
            if (!isset($entry['code'], $entry['n'])) continue;
            if (isset($codes[$entry['code']])) {
                fwrite(STDERR, "code {$entry['code']} appears in more than one manifest; keeping " . $codes[$entry['code']]['file'] . "\n");
                continue;
            }
            $codes[$entry['code']] = [
                'edition_id' => (string) ($manifest['editionId'] ?? ''),
                'n' => (int) $entry['n'],
                'file' => basename($file),
                'status' => (string) ($entry['status'] ?? ''),
            ];
        }
    }
    return $codes;
}

function manifestDirFromArgs(array $argv): string
{
    foreach ($argv as $arg) {
        if (str_starts_with($arg, '--manifests=')) return substr($arg, strlen('--manifests='));
    }
    return __DIR__ . '/../generate/files/batch';
}
