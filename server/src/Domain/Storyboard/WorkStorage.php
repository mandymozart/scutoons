<?php
declare(strict_types=1);

namespace Scutoons\Domain\Storyboard;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Content\EntityKey;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Support\Time;

/**
 * The storyboard's shared work storage (#120, agents/research/STUDIO-IN-CREATOR.md §2.2 "three places", place 2): the
 * attempts a member shares for review, so collaborators can see and compare them – on the API host's own disk
 * (STORYBOARD_WORK_PATH, default storage/storyboard-work – outside the web root and the media folder, never the readers'
 * CDN), `<toon>/<chapter>/<name>`, served to content editors only (StoryboardController). Emptied per chapter after its
 * release (an admin). Stills and videos only: png, jpeg, webp, mp4, webm – checked by extension, declared type and bytes.
 */
final class WorkStorage
{
    /** extension → content type */
    public const TYPES = ['png' => 'image/png', 'jpg' => 'image/jpeg', 'jpeg' => 'image/jpeg', 'webp' => 'image/webp', 'mp4' => 'video/mp4', 'webm' => 'video/webm'];
    /** a file name: no folders, starts with a letter or digit, ends with an allowed extension */
    private const NAME = '#^[A-Za-z0-9][A-Za-z0-9._-]{0,127}\.(png|jpe?g|webp|mp4|webm)$#';

    public function __construct(private readonly Settings $settings)
    {
    }

    public function maxBytes(): int
    {
        return max(1, $this->settings->int('STORYBOARD_WORK_MAX_BYTES'));
    }

    /** The storage's folder (absolute) */
    public function root(): string
    {
        return rtrim($this->settings->path('STORYBOARD_WORK_PATH'), '/');
    }

    /**
     * 400 invalid_toon / invalid_chapter / invalid_name
     * @return array{0: string, 1: int, 2: mixed} [toon, chapter, name]
     */
    public static function check(mixed $toon, mixed $chapter, mixed $name = null): array
    {
        if (!is_string($toon) || !preg_match(EntityKey::TOON_PATTERN, $toon) || $toon === 'scutoons') throw new ApiError(400, 'invalid_toon', 'Unknown toon.');
        $chapter = is_int($chapter) ? (string) $chapter : $chapter;
        if (!is_string($chapter) || !preg_match(EntityKey::CHAPTER_NUMBER_PATTERN, $chapter)) throw new ApiError(400, 'invalid_chapter', 'chapter is a chapter number.');
        if ($name !== null && (!is_string($name) || !preg_match(self::NAME, $name))) {
            throw new ApiError(400, 'invalid_name', 'name is a file name (letters, digits, . _ -) ending in .png, .jpg, .webp, .mp4 or .webm.');
        }
        return [$toon, (int) $chapter, $name];
    }

    private function dir(string $toon, int $chapter): string
    {
        return $this->root() . "/$toon/$chapter";
    }

    /** Whether the file exists (invalid names: false) */
    public function exists(string $toon, int $chapter, string $name): bool
    {
        if (!preg_match(self::NAME, $name)) return false;
        return is_file($this->dir($toon, $chapter) . "/$name");
    }

    /**
     * Stores the uploaded temp file $tmp under its name: 201-ish `created: true`; the same bytes again → `created: false`
     * (nothing written); other bytes under a taken name → 409 exists (a changed file gets a new name).
     * 422 invalid_type (the declared Content-Type isn't the extension's) / content_mismatch (the bytes aren't it).
     * @return array{name: string, type: string, size: int, sha256: string, url: string, created: bool}
     */
    public function put(mixed $toon, mixed $chapter, mixed $name, string $tmp, string $declaredType): array
    {
        [$toon, $chapter, $name] = self::check($toon, $chapter, $name);
        $type = self::TYPES[strtolower((string) pathinfo((string) $name, PATHINFO_EXTENSION))];
        $declared = strtolower(trim(explode(';', $declaredType)[0]));
        if ($declared === 'image/jpg') $declared = 'image/jpeg';
        if ($declared !== '' && $declared !== 'application/octet-stream' && $declared !== $type) {
            throw new ApiError(422, 'invalid_type', "The file is sent as $declared, its name says $type.");
        }
        $sniffed = self::sniff($tmp);
        if ($sniffed !== $type) throw new ApiError(422, 'content_mismatch', "The file's bytes are not $type" . ($sniffed !== null ? " but $sniffed." : '.'));
        $hash = (string) hash_file('sha256', $tmp);
        $size = (int) filesize($tmp);
        $dir = $this->dir($toon, $chapter);
        $target = "$dir/$name";
        $result = ['name' => (string) $name, 'type' => $type, 'size' => $size, 'sha256' => $hash, 'url' => self::url($toon, $chapter, (string) $name)];
        if (is_file($target)) {
            if (hash_file('sha256', $target) === $hash) return $result + ['created' => false];
            throw new ApiError(409, 'exists', 'Another file has this name – a changed file gets a new name.', ['name' => $name]);
        }
        if (!is_dir($dir) && !@mkdir($dir, 0775, true) && !is_dir($dir)) throw new ApiError(500, 'storage_failed', 'The work storage folder cannot be created.');
        $part = "$dir/.$name." . bin2hex(random_bytes(4)) . '.part';
        if (!@copy($tmp, $part) || !@rename($part, $target)) {
            @unlink($part);
            throw new ApiError(500, 'storage_failed', 'The file could not be stored.');
        }
        return $result + ['created' => true];
    }

    /**
     * A stored file for serving; 404 not_found
     * @return array{path: string, type: string, size: int}
     */
    public function file(mixed $toon, mixed $chapter, mixed $name): array
    {
        [$toon, $chapter, $name] = self::check($toon, $chapter, $name);
        $path = $this->dir($toon, $chapter) . "/$name";
        if (!is_file($path)) throw new ApiError(404, 'not_found', 'No such work file.');
        return ['path' => $path, 'type' => self::TYPES[strtolower((string) pathinfo((string) $name, PATHINFO_EXTENSION))], 'size' => (int) filesize($path)];
    }

    /**
     * The files of a chapter, by name
     * @return list<array{name: string, type: string, size: int, modified_at: string, url: string}>
     */
    public function list(mixed $toon, mixed $chapter): array
    {
        [$toon, $chapter] = self::check($toon, $chapter);
        $items = [];
        foreach ($this->names($toon, $chapter) as $name) {
            $path = $this->dir($toon, $chapter) . "/$name";
            $items[] = ['name' => $name, 'type' => self::TYPES[strtolower(pathinfo($name, PATHINFO_EXTENSION))], 'size' => (int) filesize($path),
                'modified_at' => (string) Time::iso(gmdate('Y-m-d H:i:s', (int) filemtime($path))), 'url' => self::url($toon, $chapter, $name)];
        }
        return $items;
    }

    /**
     * Empties a chapter's folder (after its release; admin): every file goes
     * @return array{deleted: int, bytes: int}
     */
    public function deleteChapter(mixed $toon, mixed $chapter): array
    {
        [$toon, $chapter] = self::check($toon, $chapter);
        $dir = $this->dir($toon, $chapter);
        $deleted = 0;
        $bytes = 0;
        foreach (is_dir($dir) ? (scandir($dir) ?: []) : [] as $entry) {
            $path = "$dir/$entry";
            if ($entry === '.' || $entry === '..' || !is_file($path)) continue;
            $size = (int) filesize($path);
            if (@unlink($path)) {
                $deleted++;
                $bytes += $size;
            }
        }
        if (is_dir($dir)) @rmdir($dir);
        return ['deleted' => $deleted, 'bytes' => $bytes];
    }

    /** The API path that serves a file (relative to the API's base) */
    public static function url(string $toon, int $chapter, string $name): string
    {
        return "/api/storyboard/work/$toon/$chapter/" . rawurlencode($name);
    }

    /** @return list<string> */
    private function names(string $toon, int $chapter): array
    {
        $dir = $this->dir($toon, $chapter);
        if (!is_dir($dir)) return [];
        $names = array_values(array_filter(scandir($dir) ?: [], fn(string $n) => preg_match(self::NAME, $n) === 1 && is_file("$dir/$n")));
        sort($names);
        return $names;
    }

    /** The content type the first bytes say (png, jpeg, webp, mp4, webm), null for anything else */
    public static function sniff(string $path): ?string
    {
        $f = @fopen($path, 'rb');
        if ($f === false) return null;
        $head = (string) fread($f, 16);
        fclose($f);
        if (str_starts_with($head, "\x89PNG\r\n\x1a\n")) return 'image/png';
        if (str_starts_with($head, "\xFF\xD8\xFF")) return 'image/jpeg';
        if (str_starts_with($head, 'RIFF') && substr($head, 8, 4) === 'WEBP') return 'image/webp';
        if (substr($head, 4, 4) === 'ftyp') return 'video/mp4';
        if (str_starts_with($head, "\x1A\x45\xDF\xA3")) return 'video/webm';
        return null;
    }
}
