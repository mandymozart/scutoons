<?php
declare(strict_types=1);

namespace Scutoons\Infrastructure\AssetStorage;

/**
 * A media storage behind the asset layer (agents/ASSETS.md "Storage drivers"): the local media folder (media/, dev),
 * a bunny.net storage zone (production), later anything else (S3, R2, …). Keys are paths from the CDN root with "/"
 * separators, never a leading "/" and never ".." (StorageKey::normalize). Content never talks to a driver: it keeps
 * referencing the stable public paths; the drivers only move bytes for the asset layer.
 */
interface AssetStorage
{
    /** short id: local | bunny | … */
    public function name(): string;

    /** a human description of where the files are (no secrets): a folder, a zone + host */
    public function describe(): string;

    /**
     * Every file below $prefix (recursive), in no particular order. $prefix '' = everything; "catpope/" = one folder.
     * @return iterable<StoredFile>
     */
    public function list(string $prefix = ''): iterable;

    /** The file's size etc., null when it doesn't exist */
    public function stat(string $key): ?StoredFile;

    /** Copies the file into a local file (overwritten) – for hashing, probing, transcoding */
    public function download(string $key, string $localFile): void;

    /** Stores a local file under $key (overwrites) and returns the stored file */
    public function upload(string $key, string $localFile, ?string $mime = null): StoredFile;

    public function delete(string $key): void;

    /** Copies a stored file to another key (overwrites the target) */
    public function copy(string $from, string $to): StoredFile;

    /** The public URL readers load ('' when the driver has no public base configured) */
    public function publicUrl(string $key): string;

    /**
     * Drops the file from the CDN edge caches after a change in place. Returns false when the driver can't (no API key,
     * no CDN in front) – the caller reports it; content with stable paths keeps working, readers may see the old bytes
     * until the cache expires.
     */
    public function purge(string $key): bool;
}
