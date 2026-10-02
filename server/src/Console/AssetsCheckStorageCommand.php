<?php
declare(strict_types=1);

namespace Scutoons\Console;

use Scutoons\Infrastructure\AssetStorage\AssetStorages;
use Scutoons\Infrastructure\AssetStorage\BunnyStorage;
use Scutoons\Infrastructure\AssetStorage\StoredFile;
use Symfony\Component\Console\Attribute\AsCommand;
use Symfony\Component\Console\Command\Command;
use Symfony\Component\Console\Input\InputInterface;
use Symfony\Component\Console\Input\InputOption;
use Symfony\Component\Console\Output\OutputInterface;
use Throwable;

/**
 * The owner's check after entering the bunny.net keys (agents/ASSETS.md "Connecting bunny.net"). READ-ONLY unless
 * --write-test: which keys are set (never their values), the zone's listing (objects, size per top folder), a
 * comparison with the local CDN folder (only on bunny / only local / different size, optionally SHA-256), the pull
 * zone statistics of the last 30 days when an API key is set. --write-test puts, reads back and deletes one small file
 * below `_test/` – nothing else on the zone is ever touched.
 */
#[AsCommand(name: 'assets:check-storage', description: 'Check the media storages (bunny.net zone vs the local CDN folder) – read-only')]
final class AssetsCheckStorageCommand extends Command
{
    public function __construct(private readonly AssetStorages $storages)
    {
        parent::__construct();
    }

    protected function configure(): void
    {
        $this->addOption('prefix', null, InputOption::VALUE_REQUIRED, 'only this folder, e.g. catpope/', '')
            ->addOption('hash', null, InputOption::VALUE_NONE, 'also compare SHA-256 of files with the same size (reads every local file)')
            ->addOption('list', null, InputOption::VALUE_REQUIRED, 'print up to N differing paths per group', '25')
            ->addOption('head', null, InputOption::VALUE_REQUIRED, 'the key to HEAD through CDN_PUBLIC_URL (default: a small image on both sides)')
            ->addOption('write-test', null, InputOption::VALUE_NONE, 'upload, read back and delete one small file below _test/ (the only write)')
            ->addOption('json', null, InputOption::VALUE_NONE, 'machine-readable report');
    }

    protected function execute(InputInterface $input, OutputInterface $output): int
    {
        $json = (bool) $input->getOption('json');
        $prefix = (string) $input->getOption('prefix');
        $limit = max(0, (int) $input->getOption('list'));
        $report = ['configured' => $this->storages->configured(), 'primary' => $this->storages->primaryName()];
        $say = static function (string $line) use ($output, $json): void {
            if (!$json) $output->writeln($line);
        };

        $say('Settings (passwords and keys are never shown):');
        foreach ($report['configured'] as $name => $row) {
            $say(sprintf('  %-32s %-8s %s%s', $name, $row['status'], $row['value'] ?? '', $row['note'] !== '' ? ($row['value'] !== null ? '  – ' : '– ') . $row['note'] : ''));
        }
        $say('Asset layer storage (ASSET_STORAGE): ' . $report['primary']);

        $local = $this->storages->local();
        $localFiles = $this->index($local->list($prefix));
        $report['local'] = ['where' => $local->describe(), ...$this->summary($localFiles)];
        $say(sprintf('Local: %s – %d files, %s', $local->describe(), count($localFiles), self::bytes($report['local']['bytes'])));

        $bunny = $this->storages->bunny();
        if (!$bunny) {
            $say('bunny.net: not configured (BUNNY_STORAGE_ZONE + BUNNY_STORAGE_READONLY_PASSWORD / BUNNY_STORAGE_PASSWORD in the repository-root .env).');
            $report['bunny'] = null;
            if ($json) $output->writeln((string) json_encode($report, JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES));
            return self::FAILURE;
        }

        $say('bunny.net: ' . $bunny->describe());
        try {
            $root = $bunny->listDirectory('');
        } catch (Throwable $error) {
            $say('  FAILED to list the zone root: ' . $error->getMessage());
            $say('  Likely: a 401 = wrong password (the zone → FTP & API Access: Password / Read-only password, not the account API key);');
            $say('          a 404 = wrong zone name or region endpoint (BUNNY_STORAGE_ENDPOINT = the zone → FTP & API Access → Hostname).');
            $report['bunny'] = ['error' => $error->getMessage()];
            if ($json) $output->writeln((string) json_encode($report, JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES));
            return self::FAILURE;
        }
        $say('  zone root: ' . implode('  ', array_map(fn(array $o) => (string) ($o['ObjectName'] ?? '?') . (!empty($o['IsDirectory']) ? '/' : ''), $root)));

        $remoteFiles = $this->index($bunny->list($prefix));
        $report['bunny'] = ['where' => $bunny->describe(), ...$this->summary($remoteFiles)];
        $say(sprintf('  %d files, %s', count($remoteFiles), self::bytes($report['bunny']['bytes'])));
        foreach ($report['bunny']['folders'] as $folder => $row) $say(sprintf('    %-24s %5d files  %s', $folder, $row['files'], self::bytes($row['bytes'])));

        // compare
        $onlyRemote = array_values(array_diff(array_keys($remoteFiles), array_keys($localFiles)));
        $onlyLocal = array_values(array_diff(array_keys($localFiles), array_keys($remoteFiles)));
        $sizeDiffers = [];
        $hashDiffers = [];
        foreach ($remoteFiles as $key => $remote) {
            $mine = $localFiles[$key] ?? null;
            if (!$mine) continue;
            if ($mine->size !== $remote->size) {
                $sizeDiffers[] = $key;
            } elseif ($input->getOption('hash') && $remote->sha256 !== null) {
                $hash = hash_file('sha256', $local->localPath($key));
                if ($hash !== $remote->sha256) $hashDiffers[] = $key;
            }
        }
        sort($onlyRemote);
        sort($onlyLocal);
        $report['compare'] = ['same_path' => count($remoteFiles) - count($onlyRemote), 'only_bunny' => $onlyRemote, 'only_local' => $onlyLocal, 'size_differs' => $sizeDiffers, 'hash_differs' => $input->getOption('hash') ? $hashDiffers : null];
        $say(sprintf('Compare (%s): %d paths on both, %d only on bunny, %d only local, %d with a different size%s',
            $prefix === '' ? 'everything' : $prefix, $report['compare']['same_path'], count($onlyRemote), count($onlyLocal), count($sizeDiffers),
            $input->getOption('hash') ? ', ' . count($hashDiffers) . ' with a different SHA-256' : ''));
        foreach (['only on bunny' => $onlyRemote, 'only local' => $onlyLocal, 'different size' => $sizeDiffers, 'different SHA-256' => $hashDiffers] as $label => $list) {
            if (!$list || $limit === 0) continue;
            $say("  $label:");
            foreach (array_slice($list, 0, $limit) as $key) $say("    $key");
            if (count($list) > $limit) $say('    … ' . (count($list) - $limit) . ' more');
        }

        // does the pull zone serve a file? (HEAD through CDN_PUBLIC_URL, no credentials)
        $report['public'] = null;
        $sample = $input->getOption('head') ?: $this->sample(array_keys(array_intersect_key($remoteFiles, $localFiles)) ?: array_keys($remoteFiles));
        if ($sample !== null && $sample !== '') {
            try {
                $head = $bunny->headPublic((string) $sample);
                $report['public'] = $head + ['expected_size' => isset($remoteFiles[$sample]) ? $remoteFiles[$sample]->size : null];
                $say(sprintf('Public URL check: HEAD %s → HTTP %d%s%s', $head['url'] ?: '(no CDN_PUBLIC_URL)', $head['status'],
                    $head['size'] !== null ? ', ' . $head['size'] . ' bytes' . (isset($remoteFiles[$sample]) && $remoteFiles[$sample]->size === $head['size'] ? ' (= the zone)' : ' (zone: ' . ($remoteFiles[$sample]->size ?? '?') . ')') : '',
                    $head['cache'] ? ', cache ' . $head['cache'] : ''));
            } catch (Throwable $error) {
                $say('Public URL check: FAILED – ' . $error->getMessage());
                $report['public'] = ['error' => $error->getMessage()];
            }
        }

        // statistics (read-only Core API call)
        $report['statistics'] = null;
        if ($bunny->hasApiKey()) {
            try {
                $stats = $bunny->statistics(gmdate('Y-m-d', time() - 30 * 86400), gmdate('Y-m-d'));
                $report['statistics'] = $stats === null ? null : [
                    'bandwidth' => (int) ($stats['TotalBandwidthUsed'] ?? 0),
                    'requests' => (int) ($stats['TotalRequestsServed'] ?? 0),
                    'cache_hit_rate' => (float) ($stats['CacheHitRate'] ?? 0),
                ];
                if ($report['statistics']) $say(sprintf('Pull zone, last 30 days: %s served, %d requests, cache hit rate %.1f %%%s',
                    self::bytes($report['statistics']['bandwidth']), $report['statistics']['requests'], $report['statistics']['cache_hit_rate'],
                    $bunny->pullZoneId() === '' ? ' (all pull zones – set BUNNY_PULLZONE_ID for one)' : ''));
            } catch (Throwable $error) {
                $say('Statistics: FAILED – ' . $error->getMessage() . ' (check BUNNY_API_KEY = the account API key)');
                $report['statistics'] = ['error' => $error->getMessage()];
            }
        } else {
            $say('Statistics / purge: off – no BUNNY_API_KEY (account → API key). Uploads work without it; after a replace in place readers may see the old file until the edge cache expires.');
        }

        if ($input->getOption('write-test')) {
            $report['write_test'] = $this->writeTest($bunny);
            $say('Write test below _test/: ' . ($report['write_test']['ok'] ? 'ok (put, listed, read back identical, deleted)' : 'FAILED – ' . $report['write_test']['error']));
        }

        if ($json) $output->writeln((string) json_encode($report, JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES));
        return self::SUCCESS;
    }

    /** @param list<string> $keys a small, stable sample: the first image, else the first key */
    private function sample(array $keys): ?string
    {
        sort($keys);
        foreach ($keys as $key) if (preg_match('/\.(webp|png|jpe?g|svg)$/i', $key)) return $key;
        return $keys[0] ?? null;
    }

    /** @return array{ok: bool, error: ?string, key: string} */
    private function writeTest(BunnyStorage $bunny): array
    {
        $key = '_test/asset-check-' . gmdate('Ymd-His') . '-' . bin2hex(random_bytes(3)) . '.txt';
        $tmp = (string) tempnam(sys_get_temp_dir(), 'asset-check-');
        $back = $tmp . '.back';
        file_put_contents($tmp, 'SCUTOONS asset layer write test ' . gmdate('c') . "\n");
        try {
            $bunny->upload($key, $tmp, 'text/plain');
            $stat = $bunny->stat($key);
            if (!$stat || $stat->size !== filesize($tmp)) throw new \RuntimeException('the uploaded file is not listed with its size');
            $bunny->download($key, $back);
            if (hash_file('sha256', $back) !== hash_file('sha256', $tmp)) throw new \RuntimeException('read back different bytes');
            return ['ok' => true, 'error' => null, 'key' => $key];
        } catch (Throwable $error) {
            return ['ok' => false, 'error' => $error->getMessage(), 'key' => $key];
        } finally {
            try {
                $bunny->delete($key);
            } catch (Throwable) {
                // reported by the next run (only_bunny lists it)
            }
            @unlink($tmp);
            @unlink($back);
        }
    }

    /**
     * @param iterable<StoredFile> $files
     * @return array<string, StoredFile>
     */
    private function index(iterable $files): array
    {
        $out = [];
        foreach ($files as $file) $out[$file->key] = $file;
        return $out;
    }

    /**
     * @param array<string, StoredFile> $files
     * @return array{files: int, bytes: int, folders: array<string, array{files: int, bytes: int}>}
     */
    private function summary(array $files): array
    {
        $folders = [];
        $bytes = 0;
        foreach ($files as $key => $file) {
            $top = str_contains($key, '/') ? substr($key, 0, (int) strpos($key, '/')) . '/' : '(root)';
            $folders[$top] ??= ['files' => 0, 'bytes' => 0];
            $folders[$top]['files']++;
            $folders[$top]['bytes'] += $file->size;
            $bytes += $file->size;
        }
        ksort($folders);
        return ['files' => count($files), 'bytes' => $bytes, 'folders' => $folders];
    }

    public static function bytes(int $bytes): string
    {
        if ($bytes < 1024) return "$bytes B";
        $value = (float) $bytes;
        foreach (['KB', 'MB', 'GB', 'TB'] as $unit) {
            $value /= 1024;
            if ($value < 1024 || $unit === 'TB') return sprintf('%.1f %s', $value, $unit);
        }
        return "$bytes B";
    }
}
