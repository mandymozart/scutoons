<?php
declare(strict_types=1);

namespace Qr;

/**
 * A tiny key/value cache: APCu when available, else one JSON file per key in CACHE_DIR. Every entry remembers when it
 * was stored, so one entry answers both "fresh enough to use" (get with a short max age) and "good enough for the
 * fallback page" (get with a long max age). Entries are dropped after $keep seconds.
 */
final class Cache
{
    private const PREFIX = 'scutoons-qr:';

    private readonly bool $apcu;

    public function __construct(private readonly string $dir, ?bool $useApcu = null)
    {
        $this->apcu = $useApcu ?? (function_exists('apcu_enabled') && apcu_enabled());
    }

    public static function fromConfig(Config $config): self
    {
        return new self($config->path('CACHE_DIR'));
    }

    public function driver(): string
    {
        return $this->apcu ? 'apcu' : 'file';
    }

    /** The value stored under $key if it is at most $maxAge seconds old, else null */
    public function get(string $key, int $maxAge): mixed
    {
        $entry = $this->apcu ? apcu_fetch(self::PREFIX . $key) : $this->readFile($key);
        if (!is_array($entry) || !isset($entry['t']) || !array_key_exists('v', $entry)) return null;
        return time() - (int) $entry['t'] <= $maxAge ? $entry['v'] : null;
    }

    public function set(string $key, mixed $value, int $keep): void
    {
        $entry = ['t' => time(), 'v' => $value];
        if ($this->apcu) {
            apcu_store(self::PREFIX . $key, $entry, $keep);
            return;
        }
        if (!is_dir($this->dir) && !@mkdir($this->dir, 0770, true) && !is_dir($this->dir)) return;
        $file = $this->file($key);
        $tmp = $file . '.' . bin2hex(random_bytes(4)) . '.tmp';
        if (@file_put_contents($tmp, json_encode($entry, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE)) === false) return;
        if (!@rename($tmp, $file)) @unlink($tmp);
        if (random_int(1, 100) === 1) $this->prune($keep);
    }

    /** @return array<string, mixed>|null */
    private function readFile(string $key): ?array
    {
        $raw = @file_get_contents($this->file($key));
        if ($raw === false) return null;
        $entry = json_decode($raw, true);
        return is_array($entry) ? $entry : null;
    }

    private function file(string $key): string
    {
        return $this->dir . '/' . sha1($key) . '.json';
    }

    /** Deletes cache files older than $keep seconds (now and then, from set()) */
    private function prune(int $keep): void
    {
        foreach (glob($this->dir . '/*.json') ?: [] as $file) {
            if (@filemtime($file) < time() - $keep) @unlink($file);
        }
    }
}
