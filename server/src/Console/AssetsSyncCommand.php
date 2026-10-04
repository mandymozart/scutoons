<?php
declare(strict_types=1);

namespace Scutoons\Console;

use Scutoons\Infrastructure\AssetStorage\AssetStorage;
use Scutoons\Infrastructure\AssetStorage\AssetStorages;
use Scutoons\Infrastructure\AssetStorage\BunnyStorage;
use Scutoons\Infrastructure\AssetStorage\LocalStorage;
use Scutoons\Infrastructure\AssetStorage\StorageKey;
use Scutoons\Infrastructure\AssetStorage\StoredFile;
use Symfony\Component\Console\Attribute\AsCommand;
use Symfony\Component\Console\Command\Command;
use Symfony\Component\Console\Input\InputInterface;
use Symfony\Component\Console\Input\InputOption;
use Symfony\Component\Console\Output\OutputInterface;
use Throwable;

/**
 * Copies media from one storage to another (agents/ASSETS.md "Migration"): e.g. local media/ → the bunny.net zone.
 * A DRY RUN by default – it prints the plan: files missing on the target (upload), files whose size differs (would be
 * overwritten – only with --overwrite), files only on the target (never touched: this command never deletes).
 * --apply performs the uploads (and with --overwrite the replacements, each followed by a purge when possible); every
 * upload is verified (listed size + SHA-256 when the target reports it) and retried up to 3 times; --purge-zone purges
 * the whole pull zone afterwards; --log writes the run's summary (no secrets) as JSON.
 * --source-root reads the source from another local folder (e.g. the main checkout's media/ – read-only).
 * Skipped always: dot files / dot folders, Thumbs.db, desktop.ini, half-written `*.uploading-*` files.
 */
#[AsCommand(name: 'assets:sync', description: 'Plan (default) or perform a copy of media between storages – never deletes')]
final class AssetsSyncCommand extends Command
{
    public function __construct(private readonly AssetStorages $storages)
    {
        parent::__construct();
    }

    protected function configure(): void
    {
        $this->addOption('from', null, InputOption::VALUE_REQUIRED, 'source storage', 'local')
            ->addOption('to', null, InputOption::VALUE_REQUIRED, 'target storage', 'bunny')
            ->addOption('prefix', null, InputOption::VALUE_REQUIRED, 'only this folder, e.g. catpope/', '')
            ->addOption('list', null, InputOption::VALUE_REQUIRED, 'print up to N paths per group (0 = none, -1 = all)', '30')
            ->addOption('apply', null, InputOption::VALUE_NONE, 'perform the uploads (without it: dry run, nothing is written)')
            ->addOption('overwrite', null, InputOption::VALUE_NONE, 'with --apply: also replace target files whose size differs')
            ->addOption('json', null, InputOption::VALUE_NONE, 'machine-readable plan')
            ->addOption('source-root', null, InputOption::VALUE_REQUIRED, 'a local folder as the source (instead of --from)')
            ->addOption('hash', null, InputOption::VALUE_NONE, 'compare SHA-256 too when the target reports checksums (same size, other bytes = overwrite)')
            ->addOption('purge-zone', null, InputOption::VALUE_NONE, 'with --apply: purge the whole bunny.net pull zone at the end (account API key)')
            ->addOption('log', null, InputOption::VALUE_REQUIRED, 'write the summary as JSON into this file');
    }

    protected function execute(InputInterface $input, OutputInterface $output): int
    {
        $root = (string) $input->getOption('source-root');
        $from = $root !== '' ? new LocalStorage($root) : $this->storages->byName((string) $input->getOption('from'));
        $to = $this->storages->byName((string) $input->getOption('to'));
        $prefix = (string) $input->getOption('prefix');
        $limit = (int) $input->getOption('list');
        $apply = (bool) $input->getOption('apply');
        $json = (bool) $input->getOption('json');
        $say = static function (string $line) use ($output, $json): void {
            if (!$json) $output->writeln($line);
        };

        $source = array_filter($this->index($from->list($prefix)), fn(StoredFile $f) => !StorageKey::junk($f->key));
        $target = $this->index($to->list($prefix));
        $upload = array_diff_key($source, $target);
        $differs = [];
        foreach (array_intersect_key($source, $target) as $key => $file) {
            if ($file->size !== $target[$key]->size) {
                $differs[$key] = $file;
            } elseif ($input->getOption('hash') && $target[$key]->sha256 !== null && $from instanceof LocalStorage
                && hash_file('sha256', $from->localPath($key)) !== $target[$key]->sha256) {
                $differs[$key] = $file;
            }
        }
        $onlyTarget = array_diff_key($target, $source);
        ksort($upload);
        ksort($differs);
        ksort($onlyTarget);

        $plan = [
            'from' => $from->describe(), 'to' => $to->describe(), 'prefix' => $prefix, 'dry_run' => !$apply,
            'upload' => $this->summary($upload), 'overwrite' => $this->summary($differs), 'untouched_only_on_target' => $this->summary($onlyTarget),
            'same' => count($source) - count($upload) - count($differs),
        ];
        $say(($apply ? 'SYNC' : 'DRY RUN – nothing is written') . ": {$plan['from']} → {$plan['to']}" . ($prefix !== '' ? " ($prefix)" : ''));
        $say(sprintf('  same path + size: %d files', $plan['same']));
        foreach (['upload' => [$upload, 'to upload (missing on the target)'], 'overwrite' => [$differs, 'different size (replaced only with --overwrite)'], 'untouched_only_on_target' => [$onlyTarget, 'only on the target (never touched)']] as $name => [$files, $label]) {
            $sum = $plan[$name];
            $say(sprintf('  %-48s %5d files  %s', $label, $sum['files'], AssetsCheckStorageCommand::bytes($sum['bytes'])));
            foreach ($sum['folders'] as $folder => $row) $say(sprintf('      %-30s %5d files  %s', $folder, $row['files'], AssetsCheckStorageCommand::bytes($row['bytes'])));
            if ($limit !== 0) {
                $keys = array_keys($files);
                foreach ($limit < 0 ? $keys : array_slice($keys, 0, $limit) as $key) $say("      $key");
                if ($limit > 0 && count($keys) > $limit) $say('      … ' . (count($keys) - $limit) . ' more');
            }
        }
        if ($json) {
            $plan['paths'] = ['upload' => array_keys($upload), 'overwrite' => array_keys($differs), 'untouched_only_on_target' => array_keys($onlyTarget)];
            $output->writeln((string) json_encode($plan, JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES));
        }
        if (!$apply) {
            $this->log($input, $plan + ['result' => null]);
            return self::SUCCESS;
        }

        $failed = [];
        $done = 0;
        $bytes = 0;
        $todo = $upload + ($input->getOption('overwrite') ? $differs : []);
        $started = time();
        foreach ($todo as $key => $file) {
            $error = null;
            for ($attempt = 1; $attempt <= 3; $attempt++) {
                try {
                    $this->copyAcross($from, $to, $key, $file);
                    $error = null;
                    break;
                } catch (Throwable $e) {
                    $error = $e->getMessage();
                    usleep(500000 * $attempt);
                }
            }
            if ($error !== null) {
                $failed[$key] = $error;
                $say("  FAILED $key: $error");
                continue;
            }
            $done++;
            $bytes += $file->size;
            $say(sprintf('  %s %s (%s)', isset($differs[$key]) ? 'replaced' : 'uploaded', $key, AssetsCheckStorageCommand::bytes($file->size)));
        }
        $purged = null;
        if ($input->getOption('purge-zone')) {
            $purged = $to instanceof BunnyStorage ? $to->purgeZone() : true;
            $say('Pull zone purge: ' . ($purged ? 'done' : 'not possible (no BUNNY_API_KEY / pull zone id)'));
        }
        $result = ['uploaded' => $done, 'bytes' => $bytes, 'failed' => $failed, 'seconds' => time() - $started, 'zone_purged' => $purged];
        $this->log($input, $plan + ['result' => $result]);
        $say(sprintf('Done: %d uploaded (%s), %d failed, %d s.', $done, AssetsCheckStorageCommand::bytes($bytes), count($failed), $result['seconds']));
        return $failed ? self::FAILURE : self::SUCCESS;
    }

    /** @param array<string, mixed> $data */
    private function log(InputInterface $input, array $data): void
    {
        $file = (string) $input->getOption('log');
        if ($file === '') return;
        $dir = dirname($file);
        if (!is_dir($dir)) @mkdir($dir, 0775, true);
        file_put_contents($file, json_encode(['at' => gmdate('c')] + $data, JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES) . "\n");
    }

    /** Copies one file and verifies it on the target (listed size; SHA-256 when the target reports one) */
    private function copyAcross(AssetStorage $from, AssetStorage $to, string $key, StoredFile $file): void
    {
        $tmp = (string) tempnam(sys_get_temp_dir(), 'asset-sync-');
        try {
            $from->download($key, $tmp);
            $hash = (string) hash_file('sha256', $tmp);
            $to->upload($key, $tmp, $file->mime);
            $stored = $to->stat($key);
            if (!$stored) throw new \RuntimeException('not listed after the upload');
            if ($stored->size !== filesize($tmp)) throw new \RuntimeException("size on the target {$stored->size} ≠ " . filesize($tmp));
            if ($stored->sha256 !== null && $stored->sha256 !== $hash) throw new \RuntimeException('checksum on the target differs');
        } finally {
            @unlink($tmp);
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
            $parts = explode('/', $key);
            $folder = count($parts) > 2 ? $parts[0] . '/' . $parts[1] . '/' : (count($parts) === 2 ? $parts[0] . '/' : '(root)');
            $folders[$folder] ??= ['files' => 0, 'bytes' => 0];
            $folders[$folder]['files']++;
            $folders[$folder]['bytes'] += $file->size;
            $bytes += $file->size;
        }
        ksort($folders);
        return ['files' => count($files), 'bytes' => $bytes, 'folders' => $folders];
    }
}
