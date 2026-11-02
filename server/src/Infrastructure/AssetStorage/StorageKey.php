<?php
declare(strict_types=1);

namespace Scutoons\Infrastructure\AssetStorage;

use InvalidArgumentException;

/** Storage keys: CDN-root paths with "/" separators, no leading "/", no "." / ".." segments, no control characters */
final class StorageKey
{
    /** A clean key or an InvalidArgumentException; $allowEmpty for list prefixes */
    public static function normalize(string $key, bool $allowEmpty = false): string
    {
        $key = trim(str_replace('\\', '/', $key));
        $key = (string) preg_replace('#/+#', '/', $key);
        $key = ltrim($key, '/');
        if ($key === '') {
            if ($allowEmpty) return '';
            throw new InvalidArgumentException('Empty storage key.');
        }
        if (preg_match('/[\x00-\x1f\x7f]/', $key)) throw new InvalidArgumentException('Control characters in a storage key.');
        foreach (explode('/', rtrim($key, '/')) as $segment) {
            if ($segment === '.' || $segment === '..') throw new InvalidArgumentException("Invalid storage key: $key");
        }
        if (strlen($key) > 500) throw new InvalidArgumentException('Storage key too long.');
        return $key;
    }

    /** true when the key is valid (see normalize) */
    public static function valid(string $key): bool
    {
        try {
            self::normalize($key);
            return true;
        } catch (InvalidArgumentException) {
            return false;
        }
    }

    /** Folders the platform itself writes (compiled rig modules `modules/<hash>.wasm`, #281): not assets – never imported or uploaded to */
    public const SYSTEM_PREFIXES = ['modules/'];

    public static function system(string $key): bool
    {
        foreach (self::SYSTEM_PREFIXES as $prefix) {
            if (str_starts_with($key, $prefix)) return true;
        }
        return false;
    }

    /** OS junk, dot files / dot folders and half-written uploads – never synced, never registered */
    public static function junk(string $key): bool
    {
        $name = strtolower(self::basename($key));
        return in_array($name, ['thumbs.db', 'desktop.ini', '.ds_store'], true) || str_contains($name, '.uploading-')
            || preg_match('#(^|/)\.#', $key) === 1;
    }

    /** "catpope/videos/a.mp4" → "a.mp4" */
    public static function basename(string $key): string
    {
        $pos = strrpos($key, '/');
        return $pos === false ? $key : substr($key, $pos + 1);
    }

    /** "catpope/videos/a.mp4" → "catpope/videos" ('' at the root) */
    public static function dirname(string $key): string
    {
        $pos = strrpos($key, '/');
        return $pos === false ? '' : substr($key, 0, $pos);
    }

    /** Mime type by extension (no fileinfo extension needed; the storages store what we send) */
    public static function mime(string $key): string
    {
        $ext = strtolower(pathinfo($key, PATHINFO_EXTENSION));
        return match ($ext) {
            'jpg', 'jpeg' => 'image/jpeg',
            'png' => 'image/png',
            'webp' => 'image/webp',
            'gif' => 'image/gif',
            'svg' => 'image/svg+xml',
            'avif' => 'image/avif',
            'mp4', 'm4v' => 'video/mp4',
            'webm' => 'video/webm',
            'mov' => 'video/quicktime',
            'mp3' => 'audio/mpeg',
            'm4a', 'aac' => 'audio/mp4',
            'ogg', 'oga' => 'audio/ogg',
            'wav' => 'audio/wav',
            'flac' => 'audio/flac',
            'zip' => 'application/zip',
            'pdf' => 'application/pdf',
            'glb' => 'model/gltf-binary',
            'wasm' => 'application/wasm',
            'json' => 'application/json',
            'mscz' => 'application/octet-stream',
            default => 'application/octet-stream',
        };
    }
}
