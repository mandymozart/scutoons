<?php
declare(strict_types=1);

namespace Scutoons\Infrastructure\Cache;

/**
 * Files below CACHE_PATH (default storage/cache – private, never overwritten by a deploy): one file per key, written
 * atomically (temp file + rename). Entries are content-addressed, so an old file is only garbage: now and then a write
 * removes files not touched for `ttl` seconds. Any I/O problem is a cache miss, never an error.
 */
final class FileCache implements Cache
{
    /** 1 in PRUNE_EVERY writes sweeps the folder */
    private const PRUNE_EVERY = 200;

    public function __construct(private readonly string $directory)
    {
    }

    public function get(string $key): ?string
    {
        $value = @file_get_contents($this->file($key));
        return $value === false ? null : $value;
    }

    public function set(string $key, string $value, int $ttl): void
    {
        if (!is_dir($this->directory) && !@mkdir($this->directory, 0775, true) && !is_dir($this->directory)) return;
        $file = $this->file($key);
        $tmp = $file . '.' . bin2hex(random_bytes(4)) . '.tmp';
        if (@file_put_contents($tmp, $value) === false) return;
        if (!@rename($tmp, $file)) @unlink($tmp);
        if (random_int(1, self::PRUNE_EVERY) === 1) $this->prune($ttl);
    }

    /** Removes entries older than `ttl` seconds (and leftover temp files) */
    public function prune(int $ttl): int
    {
        $removed = 0;
        $limit = time() - max(60, $ttl);
        foreach (glob($this->directory . '/*') ?: [] as $file) {
            if (is_file($file) && (int) @filemtime($file) < $limit && @unlink($file)) $removed++;
        }
        return $removed;
    }

    private function file(string $key): string
    {
        return $this->directory . '/' . hash('sha256', $key) . '.cache';
    }
}
