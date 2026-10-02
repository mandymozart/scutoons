<?php
declare(strict_types=1);

namespace Scutoons\Infrastructure\Storage;

use RuntimeException;

/**
 * Private files of the admin tools (FILES_PATH, default storage/files – never served directly):
 *   batch/         BATCH-<timestamp>-<edition>.json   batch manifests
 *   svgs/          SCUTOONS-QR-MEMBERSHIP-<code>.svg  QR codes
 *   certificates/  SCUTOONS-COA-<code>.pdf            certificates of authenticity
 *   signatures/    SIGNATURE-<code>.json              signatures (also in qr.signature)
 *   zips/          BATCH-….zip                        batch downloads
 *   claims/        <claim id>-<file id>.<ext>          evidence of ownership claims (jpg, png, webp, pdf) – only through
 *                                                    the claims methods below (never the admin files endpoint)
 * Only whitelisted folders and safe file names (no path separators, no leading dot).
 */
final class FileStore
{
    public const TYPES = ['certificates', 'svgs', 'signatures', 'batch', 'zips'];
    public const NAME_PATTERN = '/^[A-Za-z0-9_.-]{1,200}\.(pdf|svg|json|zip)$/';

    public const CLAIMS = 'claims';
    public const CLAIM_NAME_PATTERN = '/^[0-9a-f-]{36}-[0-9a-f]{8,36}\.(jpg|png|webp|pdf)$/';

    public function __construct(private readonly string $root)
    {
    }

    public function root(): string
    {
        return $this->root;
    }

    public static function validName(string $type, string $filename): bool
    {
        return in_array($type, self::TYPES, true) && preg_match(self::NAME_PATTERN, $filename) === 1
            && !str_starts_with($filename, '.') && basename($filename) === $filename;
    }

    /** Absolute path of a stored file (the name must be valid) */
    public function path(string $type, string $filename): string
    {
        if (!self::validName($type, $filename)) throw new RuntimeException("Invalid file $type/$filename");
        return $this->root . '/' . $type . '/' . $filename;
    }

    public function exists(string $type, string $filename): bool
    {
        return self::validName($type, $filename) && is_file($this->path($type, $filename));
    }

    public function read(string $type, string $filename): ?string
    {
        if (!$this->exists($type, $filename)) return null;
        $content = file_get_contents($this->path($type, $filename));
        return $content === false ? null : $content;
    }

    public function write(string $type, string $filename, string $content): void
    {
        $this->ensureDir($type);
        if (file_put_contents($this->path($type, $filename), $content, LOCK_EX) === false) {
            throw new RuntimeException("Could not write $type/$filename");
        }
    }

    public function ensureDir(string $type): string
    {
        if (!in_array($type, self::TYPES, true)) throw new RuntimeException("Unknown file type $type");
        $dir = $this->root . '/' . $type;
        if (!is_dir($dir) && !@mkdir($dir, 0770, true) && !is_dir($dir)) throw new RuntimeException("Could not create $dir");
        return $dir;
    }

    /** @return list<string> file names in a folder matching a glob pattern (e.g. BATCH-*.json), sorted */
    public function list(string $type, string $pattern = '*'): array
    {
        $dir = $this->root . '/' . $type;
        if (!in_array($type, self::TYPES, true) || !is_dir($dir)) return [];
        $files = array_map('basename', glob($dir . '/' . $pattern) ?: []);
        sort($files);
        return $files;
    }

    /** Absolute path of an ownership-claim evidence file (agents/ITEMS.md "Ownership, claims, gifting") */
    public function claimPath(string $filename): string
    {
        if (preg_match(self::CLAIM_NAME_PATTERN, $filename) !== 1) throw new RuntimeException("Invalid claim file $filename");
        return $this->root . '/' . self::CLAIMS . '/' . $filename;
    }

    public function writeClaim(string $filename, string $content): void
    {
        $dir = $this->root . '/' . self::CLAIMS;
        if (!is_dir($dir) && !@mkdir($dir, 0770, true) && !is_dir($dir)) throw new RuntimeException("Could not create $dir");
        if (file_put_contents($this->claimPath($filename), $content, LOCK_EX) === false) throw new RuntimeException("Could not write claims/$filename");
    }

    public function readClaim(string $filename): ?string
    {
        $path = $this->claimPath($filename);
        if (!is_file($path)) return null;
        $content = file_get_contents($path);
        return $content === false ? null : $content;
    }

    public function deleteClaim(string $filename): void
    {
        $path = $this->claimPath($filename);
        if (is_file($path)) @unlink($path);
    }
}
