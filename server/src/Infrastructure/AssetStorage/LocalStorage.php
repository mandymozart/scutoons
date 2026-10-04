<?php
declare(strict_types=1);

namespace Scutoons\Infrastructure\AssetStorage;

use FilesystemIterator;
use RecursiveDirectoryIterator;
use RecursiveIteratorIterator;
use RuntimeException;
use SplFileInfo;

/**
 * The local media folder (media/ – what scripts/dev/media-serve.mjs serves on :8001):
 * the development storage, and the test double of every other driver. Purging is a no-op (no cache in front).
 * Dot files and dot folders (.htaccess, .assets-trash) are not listed.
 */
final class LocalStorage implements AssetStorage
{
    private readonly string $root;

    public function __construct(string $root, private readonly string $publicBase = '')
    {
        $this->root = rtrim(str_replace('\\', '/', $root), '/');
    }

    public function name(): string
    {
        return 'local';
    }

    public function describe(): string
    {
        return 'local folder ' . $this->root;
    }

    public function root(): string
    {
        return $this->root;
    }

    private function path(string $key): string
    {
        return $this->root . '/' . StorageKey::normalize($key);
    }

    public function list(string $prefix = ''): iterable
    {
        $prefix = StorageKey::normalize($prefix, true);
        $base = $prefix === '' ? $this->root : $this->root . '/' . rtrim($prefix, '/');
        if (is_file($base)) {
            $file = $this->stat($prefix);
            if ($file) yield $file;
            return;
        }
        if (!is_dir($base)) return;
        $iterator = new RecursiveIteratorIterator(
            new \RecursiveCallbackFilterIterator(
                new RecursiveDirectoryIterator($base, FilesystemIterator::SKIP_DOTS | FilesystemIterator::UNIX_PATHS),
                static fn(SplFileInfo $file): bool => !str_starts_with($file->getFilename(), '.'),
            ),
        );
        $length = strlen($this->root) + 1;
        /** @var SplFileInfo $file */
        foreach ($iterator as $file) {
            if (!$file->isFile()) continue;
            $key = substr(str_replace('\\', '/', $file->getPathname()), $length);
            yield new StoredFile($key, (int) $file->getSize(), gmdate('c', (int) $file->getMTime()), null, StorageKey::mime($key));
        }
    }

    public function stat(string $key): ?StoredFile
    {
        $path = $this->path($key);
        if (!is_file($path)) return null;
        clearstatcache(true, $path);
        $key = StorageKey::normalize($key);
        return new StoredFile($key, (int) filesize($path), gmdate('c', (int) filemtime($path)), null, StorageKey::mime($key));
    }

    public function download(string $key, string $localFile): void
    {
        $path = $this->path($key);
        if (!is_file($path)) throw new RuntimeException("Not on the storage: $key");
        if (!@copy($path, $localFile)) throw new RuntimeException("Could not copy $key");
    }

    public function upload(string $key, string $localFile, ?string $mime = null): StoredFile
    {
        $path = $this->path($key);
        if (!is_file($localFile)) throw new RuntimeException('Upload source missing.');
        $dir = dirname($path);
        if (!is_dir($dir) && !@mkdir($dir, 0775, true) && !is_dir($dir)) throw new RuntimeException("Could not create the folder of $key");
        // write next to the target, then rename: readers never see a half-written file
        $tmp = $path . '.uploading-' . bin2hex(random_bytes(4));
        if (!@copy($localFile, $tmp)) throw new RuntimeException("Could not write $key");
        if (is_file($path)) @unlink($path);
        if (!@rename($tmp, $path)) {
            @unlink($tmp);
            throw new RuntimeException("Could not store $key");
        }
        return $this->stat($key) ?? throw new RuntimeException("Stored file vanished: $key");
    }

    public function delete(string $key): void
    {
        $path = $this->path($key);
        if (is_file($path) && !@unlink($path)) throw new RuntimeException("Could not delete $key");
    }

    public function copy(string $from, string $to): StoredFile
    {
        return $this->upload($to, $this->path($from), null);
    }

    public function publicUrl(string $key): string
    {
        return $this->publicBase === '' ? '' : rtrim($this->publicBase, '/') . '/' . StorageKey::normalize($key);
    }

    public function purge(string $key): bool
    {
        return true; // no cache in front of the local folder
    }

    /** The local file of a key (for hashing / probing without a copy) */
    public function localPath(string $key): string
    {
        return $this->path($key);
    }
}
