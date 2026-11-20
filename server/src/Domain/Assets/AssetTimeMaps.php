<?php
declare(strict_types=1);

namespace Scutoons\Domain\Assets;

use RuntimeException;
use Scutoons\Infrastructure\AssetStorage\AssetStorages;
use Scutoons\Infrastructure\Config\Settings;
use Throwable;

/**
 * A derivative's time map (source time at its 0, its frame rate), measured by the core's `deck-asset-timemap <source>
 * <derivative>` (deck/media, agents/ASSETS.md "Time maps"); nothing is measured here. The binary is TIMEMAP_PATH.
 * A failed measurement is a null and a log line, never a failed job.
 */
final class AssetTimeMaps
{
    public function __construct(
        private readonly AssetRegistry $registry,
        private readonly AssetStorages $storages,
        private readonly Settings $settings,
    ) {
    }

    public function binary(): string
    {
        return $this->settings->get('TIMEMAP_PATH') ?: 'deck-asset-timemap';
    }

    /**
     * @return array{offset: float, fps: float}
     * @throws RuntimeException the reason
     */
    public function measure(string $source, string $derivative): array
    {
        $command = escapeshellarg($this->binary()) . ' ' . escapeshellarg($source) . ' ' . escapeshellarg($derivative);
        $process = @proc_open($command, [1 => ['pipe', 'w'], 2 => ['pipe', 'w']], $pipes);
        if (!is_resource($process)) throw new RuntimeException('cannot start ' . $this->binary());
        $stdout = (string) stream_get_contents($pipes[1]);
        $stderr = (string) stream_get_contents($pipes[2]);
        fclose($pipes[1]);
        fclose($pipes[2]);
        $code = proc_close($process);
        if ($code !== 0) {
            $why = trim($stderr) !== '' ? trim($stderr) : "exit code $code";
            throw new RuntimeException('timemap failed: ' . substr($why, -300));
        }
        $map = json_decode($stdout, true);
        if (!is_array($map) || !isset($map['offset'], $map['fps']) || !is_numeric($map['offset']) || !is_numeric($map['fps'])
            || !is_finite((float) $map['offset']) || !is_finite((float) $map['fps'])) {
            throw new RuntimeException('timemap printed no time map: ' . substr(trim($stdout), 0, 100));
        }
        return ['offset' => (float) $map['offset'], 'fps' => (float) $map['fps']];
    }

    /** Measures and stores a rendition's map; a failure stores NULL (a replaced file's old map is stale) and says why. True when stored. */
    public function store(int $renditionId, string $source, string $derivative, callable $note): bool
    {
        try {
            $map = $this->measure($source, $derivative);
        } catch (Throwable $error) {
            $this->registry->setTimeMap($renditionId, null, null);
            $note('time map not measured: ' . $error->getMessage());
            return false;
        }
        $this->registry->setTimeMap($renditionId, $map['offset'], $map['fps']);
        $note(sprintf('time map: offset %s s, %s fps', $map['offset'], $map['fps']));
        return true;
    }

    /**
     * Measures the renditions that have no map yet, in batches of $batch (ordered by id: --after resumes behind the
     * last one a run reached); a failure leaves NULL and the run goes on.
     * @return array{measured: int, failed: int, last: int, left: int}
     */
    public function backfill(int $batch, int $after, ?int $limit, callable $say): array
    {
        $result = ['measured' => 0, 'failed' => 0, 'last' => $after, 'left' => 0];
        $storage = $this->storages->primary();
        $work = sys_get_temp_dir() . '/asset-timemaps-' . bin2hex(random_bytes(3));
        @mkdir($work, 0775, true);
        try {
            while ($limit === null || $result['measured'] + $result['failed'] < $limit) {
                $size = $limit === null ? $batch : min($batch, $limit - $result['measured'] - $result['failed']);
                $rows = $this->registry->withoutTimeMap($result['last'], $size);
                if (!$rows) break;
                $sources = [];
                foreach ($rows as $row) {
                    $result['last'] = (int) $row['id'];
                    $path = (string) $row['path'];
                    try {
                        $versionKey = (int) $row['version_id'];
                        if (!isset($sources[$versionKey])) {
                            foreach ($sources as $old) @unlink($old);
                            $sources = [];
                            $local = "$work/source-$versionKey." . AssetPaths::extension((string) $row['source']);
                            $storage->download((string) $row['source'], $local);
                            $sources[$versionKey] = $local;
                        }
                        $file = "$work/derivative." . AssetPaths::extension($path);
                        $storage->download($path, $file);
                        $ok = $this->store((int) $row['id'], $sources[$versionKey], $file, fn(string $line) => $say("  $line"));
                        @unlink($file);
                    } catch (Throwable $error) {
                        $ok = false;
                        $say('  not measured: ' . $error->getMessage());
                    }
                    $result[$ok ? 'measured' : 'failed']++;
                    $say(($ok ? 'ok     ' : 'FAILED ') . "#{$row['id']} $path");
                }
                foreach ($sources as $old) @unlink($old);
            }
        } finally {
            foreach (glob($work . '/*') ?: [] as $file) @unlink($file);
            @rmdir($work);
        }
        $result['left'] = $this->registry->countWithoutTimeMap($result['last']);
        return $result;
    }
}
