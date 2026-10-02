<?php
declare(strict_types=1);

namespace Scutoons\Domain\Assets;

use RuntimeException;
use Scutoons\Domain\ApiError;
use Scutoons\Infrastructure\AssetStorage\AssetStorage;
use Scutoons\Infrastructure\AssetStorage\AssetStorages;
use Scutoons\Infrastructure\AssetStorage\StorageKey;
use Scutoons\Infrastructure\AssetStorage\StoredFile;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Time;
use Throwable;

/**
 * Optimization jobs (agents/ASSETS.md "Optimization"): queued from the admin's Assets page (or the CLI), run by
 * `php bin/console assets:jobs` on a machine with ffmpeg (the owner's – the shared host has none): download the
 * source from the storage, transcode, upload the outputs next to it, register them as renditions (origin generated).
 *   video-renditions  480p / 720p / 1080p H.264 (crf 24 / 24 / 22, preset slow, yuv420p, AAC 128k, +faststart) –
 *                     like the trailer loop; only levels up to the source's height; existing files are kept unless
 *                     params.overwrite (then replaced in place + purged)
 *   image-webp        a WebP twin of a PNG / JPEG source (q 85), params.widths [390, 780 …] → <slug>.<w>w.webp
 *   poster            a WebP frame of a video at params.at seconds (default 1) → <slug>.poster.webp
 */
final class AssetJobs
{
    public const KINDS = ['video-renditions', 'image-webp', 'poster'];
    /** height → [quality level, crf] */
    public const VIDEO_LEVELS = [480 => ['low', 24], 720 => ['medium', 24], 1080 => ['high', 22]];

    public function __construct(
        private readonly Database $db,
        private readonly Clock $clock,
        private readonly AssetRegistry $registry,
        private readonly AssetStorages $storages,
        private readonly MediaProbe $probe,
        private readonly Settings $settings,
    ) {
    }

    /**
     * @param array<mixed> $params
     * @return array<string, mixed>
     */
    public function enqueue(string $assetId, string $kind, array $params = [], ?string $by = null): array
    {
        $asset = $this->registry->asset($assetId) ?? throw new ApiError(404, 'not_found', 'Unknown asset.');
        if (!in_array($kind, self::KINDS, true)) throw new ApiError(422, 'invalid_kind', 'Unknown job kind.', ['kinds' => self::KINDS]);
        $expected = $kind === 'image-webp' ? 'image' : 'video';
        if ($asset['kind'] !== $expected) throw new ApiError(422, 'invalid_kind', "A $kind job needs a $expected asset.");
        $this->db->run('INSERT INTO asset_jobs (asset_id, kind, status, params, created_at, created_by) VALUES (?, ?, ?, ?, ?, ?)',
            [$assetId, $kind, 'queued', (string) json_encode($params, JSON_UNESCAPED_SLASHES), $this->clock->utc(), $by]);
        return $this->job($this->db->lastInsertId());
    }

    /** @return array<string, mixed> */
    public function job(int $id): array
    {
        $j = $this->db->one('SELECT j.*, a.slug FROM asset_jobs j LEFT JOIN assets a ON a.id = j.asset_id WHERE j.id = ?', [$id]) ?? throw new ApiError(404, 'not_found', 'Unknown job.');
        return ['id' => (int) $j['id'], 'asset' => $j['asset_id'], 'slug' => $j['slug'], 'kind' => $j['kind'], 'status' => $j['status'],
            'params' => json_decode((string) $j['params'], true), 'log' => $j['log'], 'created_at' => Time::iso($j['created_at']),
            'started_at' => Time::iso($j['started_at']), 'finished_at' => Time::iso($j['finished_at'])];
    }

    /** @return list<array<string, mixed>> */
    public function recent(int $limit = 50): array
    {
        return array_map(fn(array $r) => $this->job((int) $r['id']), $this->db->all('SELECT id FROM asset_jobs ORDER BY id DESC LIMIT ' . max(1, min(200, $limit))));
    }

    /** The next queued job's id, null when none */
    public function next(): ?int
    {
        $id = $this->db->value("SELECT id FROM asset_jobs WHERE status = 'queued' ORDER BY id LIMIT 1");
        return $id === null ? null : (int) $id;
    }

    public function ffmpeg(): string
    {
        return $this->settings->get('FFMPEG_PATH') ?: 'ffmpeg';
    }

    /**
     * Runs one job; returns it (status done | failed)
     * @return array<string, mixed>
     */
    public function run(int $id, ?callable $say = null): array
    {
        $claimed = $this->db->run("UPDATE asset_jobs SET status = 'running', started_at = ? WHERE id = ? AND status = 'queued'", [$this->clock->utc(), $id])->rowCount();
        if ($claimed === 0) throw new RuntimeException("Job $id is not queued.");
        $job = $this->job($id);
        $log = [];
        $note = static function (string $line) use (&$log, $say): void {
            $log[] = $line;
            if ($say) $say($line);
        };
        $status = 'done';
        $work = sys_get_temp_dir() . '/asset-job-' . $id . '-' . bin2hex(random_bytes(3));
        @mkdir($work, 0775, true);
        try {
            $asset = $this->registry->asset((string) $job['asset']) ?? throw new RuntimeException('The asset is gone.');
            $version = $this->registry->currentVersion((string) $asset['id']) ?? throw new RuntimeException('No current version.');
            $storage = $this->storages->primary();
            $source = (string) $version['path'];
            $local = $work . '/source.' . AssetPaths::extension($source);
            $note("source $source (" . $storage->name() . ')');
            $storage->download($source, $local);
            $params = is_array($job['params']) ? $job['params'] : [];
            $outputs = match ($job['kind']) {
                'video-renditions' => $this->videoRenditions($local, (string) $asset['slug'], $params, $work, $note),
                'image-webp' => $this->imageWebp($local, (string) $asset['slug'], $params, $work, $note),
                'poster' => $this->poster($local, (string) $asset['slug'], $params, $work, $note),
                default => throw new RuntimeException('Unknown kind.'),
            };
            $this->store($storage, (string) $asset['id'], (int) $version['id'], $outputs, ($params['overwrite'] ?? false) === true, $note);
        } catch (Throwable $error) {
            $status = 'failed';
            $note('FAILED: ' . $error->getMessage());
        } finally {
            foreach (glob($work . '/*') ?: [] as $file) @unlink($file);
            @rmdir($work);
        }
        $this->db->run('UPDATE asset_jobs SET status = ?, log = ?, finished_at = ? WHERE id = ?', [$status, implode("\n", $log), $this->clock->utc(), $id]);
        return $this->job($id);
    }

    /**
     * @param list<array{file: string, path: string, label: string, quality: ?string}> $outputs
     */
    private function store(AssetStorage $storage, string $assetId, int $versionId, array $outputs, bool $overwrite, callable $note): void
    {
        $existing = [];
        foreach ($this->registry->renditions($versionId) as $r) $existing[(string) $r['path']] = $r;
        foreach ($outputs as $out) {
            $there = $storage->stat($out['path']);
            if ($there && !$overwrite) {
                $note("kept {$out['path']} (exists – params.overwrite replaces it)");
                continue;
            }
            $stored = $storage->upload($out['path'], $out['file'], StorageKey::mime($out['path']));
            $stored = new StoredFile($stored->key, $stored->size, gmdate('c'), hash_file('sha256', $out['file']) ?: null, $stored->mime);
            $probe = $this->probe->probe($out['file'], AssetPaths::kind($out['path']));
            if (isset($existing[$out['path']])) {
                $this->registry->updateRendition((int) $existing[$out['path']]['id'], $stored, $probe);
                $this->db->run("UPDATE asset_renditions SET origin = 'generated' WHERE id = ?", [$existing[$out['path']]['id']]);
            } else {
                $this->registry->addRendition($assetId, $versionId, $out['label'], $out['quality'], $stored, $probe, 'generated');
            }
            if ($there) $storage->purge($out['path']);
            $note(sprintf('%s %s (%d bytes)', $there ? 'replaced' : 'added', $out['path'], $stored->size));
        }
        $this->registry->touch($assetId);
    }

    /** "catpope/videos/x" + "720p" + "mp4" → "catpope/videos/x.720p.mp4" */
    public static function outputPath(string $slug, string $marker, string $ext): string
    {
        return $slug . ($marker !== '' ? '.' . $marker : '') . '.' . $ext;
    }

    /**
     * @param array<mixed> $params
     * @return list<array{file: string, path: string, label: string, quality: ?string}>
     */
    private function videoRenditions(string $source, string $slug, array $params, string $work, callable $note): array
    {
        $probe = $this->probe->probe($source, 'video');
        $height = (int) ($probe['height'] ?? 0);
        $levels = isset($params['levels']) && is_array($params['levels']) ? array_map('intval', $params['levels']) : array_keys(self::VIDEO_LEVELS);
        $out = [];
        foreach ($levels as $level) {
            if (!isset(self::VIDEO_LEVELS[$level])) continue;
            if ($height > 0 && $level > $height) {
                $note("skip {$level}p (the source is {$height}p)");
                continue;
            }
            [$quality, $crf] = self::VIDEO_LEVELS[$level];
            $file = "$work/$level.mp4";
            $this->exec([$this->ffmpeg(), '-y', '-v', 'error', '-i', $source, '-vf', "scale=-2:$level", '-c:v', 'libx264', '-preset', 'slow', '-crf', (string) $crf,
                '-pix_fmt', 'yuv420p', '-c:a', 'aac', '-b:a', '128k', '-movflags', '+faststart', $file], $note);
            $out[] = ['file' => $file, 'path' => self::outputPath($slug, $level . 'p', 'mp4'), 'label' => $level . 'p', 'quality' => $quality];
        }
        return $out;
    }

    /**
     * @param array<mixed> $params
     * @return list<array{file: string, path: string, label: string, quality: ?string}>
     */
    private function imageWebp(string $source, string $slug, array $params, string $work, callable $note): array
    {
        if (AssetPaths::extension($source) === 'webp' && empty($params['widths'])) throw new RuntimeException('The source is a WebP already – give params.widths for smaller copies.');
        $out = [];
        if (AssetPaths::extension($source) !== 'webp') {
            $file = "$work/full.webp";
            $this->exec([$this->ffmpeg(), '-y', '-v', 'error', '-i', $source, '-c:v', 'libwebp', '-quality', '85', '-compression_level', '6', $file], $note);
            $out[] = ['file' => $file, 'path' => self::outputPath($slug, '', 'webp'), 'label' => 'webp', 'quality' => null];
        }
        foreach (isset($params['widths']) && is_array($params['widths']) ? $params['widths'] : [] as $width) {
            $width = (int) $width;
            if ($width < 16 || $width > 8000) continue;
            $file = "$work/w$width.webp";
            $this->exec([$this->ffmpeg(), '-y', '-v', 'error', '-i', $source, '-vf', "scale=$width:-2:flags=lanczos", '-c:v', 'libwebp', '-quality', '85', $file], $note);
            $out[] = ['file' => $file, 'path' => self::outputPath($slug, $width . 'w', 'webp'), 'label' => $width . 'w', 'quality' => null];
        }
        return $out;
    }

    /**
     * @param array<mixed> $params
     * @return list<array{file: string, path: string, label: string, quality: ?string}>
     */
    private function poster(string $source, string $slug, array $params, string $work, callable $note): array
    {
        $at = isset($params['at']) && is_numeric($params['at']) ? max(0.0, (float) $params['at']) : 1.0;
        $file = "$work/poster.webp";
        $this->exec([$this->ffmpeg(), '-y', '-v', 'error', '-ss', (string) $at, '-i', $source, '-frames:v', '1', '-c:v', 'libwebp', '-quality', '88', $file], $note);
        return [['file' => $file, 'path' => self::outputPath($slug, 'poster', 'webp'), 'label' => 'poster', 'quality' => null]];
    }

    /** @param list<string> $command */
    private function exec(array $command, callable $note): void
    {
        $line = implode(' ', array_map('escapeshellarg', $command));
        $note('$ ' . implode(' ', array_map(fn($a) => str_contains($a, sys_get_temp_dir()) ? basename($a) : $a, $command)));
        $output = [];
        $code = 1;
        exec($line . ' 2>&1', $output, $code);
        if ($code !== 0) throw new RuntimeException('ffmpeg failed (' . $code . '): ' . implode(' ', array_slice($output, -5)));
    }
}
