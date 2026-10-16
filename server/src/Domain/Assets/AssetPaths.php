<?php
declare(strict_types=1);

namespace Scutoons\Domain\Assets;

use Scutoons\Infrastructure\AssetStorage\StorageKey;

/**
 * How CDN paths map to logical assets (agents/ASSETS.md "Grouping"): files that differ only by a rendition marker or
 * by their image format are ONE asset with several renditions.
 *   videos/s1-trailer-loop.{480p,720p,1080p,1280w,1920w}.mp4   → catpope/videos/s1-trailer-loop   (480p = low …)
 *   audio/music/x.{128,320}.mp3                                → …/audio/music/x                  (128k, 320k)
 *   images/{1x,2x,3x}/<rest>                                  → …/images/<rest>                  (1x = low …)
 *   images/x@2x.png, images/x.png + images/x.webp             → …/images/x
 * The slug is the path without marker and extension; it is stable while the files exist and is what an asset is
 * found by on a re-scan. Pure functions, no storage access.
 */
final class AssetPaths
{
    public const KINDS = ['image', 'video', 'audio', 'archive', 'document', 'model', 'other'];
    public const IMAGE = ['jpg', 'jpeg', 'png', 'webp', 'gif', 'svg', 'avif'];
    public const VIDEO = ['mp4', 'webm', 'mov', 'm4v'];
    public const AUDIO = ['mp3', 'm4a', 'aac', 'ogg', 'oga', 'wav', 'flac'];
    /** 3D models: binary glTF only – one self-contained file (a .gltf points to separate .bin / textures that a move or
     * rename would break; GLB ↔ glTF converts losslessly when a model must be edited) – owner 2026-10-13 */
    public const MODEL = ['glb'];
    /** the quality level of a video height (MediaQuality, PLAN.md "one file per quality level") */
    public const VIDEO_QUALITY = [480 => 'low', 720 => 'medium', 1080 => 'high'];
    /** slice densities (IMAGE_QUALITY_DENSITIES) */
    public const DENSITY_QUALITY = ['1x' => 'low', '2x' => 'medium', '3x' => 'high'];
    /** the production CDN's public bases: cdn.scutoons.com (bunny custom hostname since 10.3) and the pull zone's own
     * hostname, which stays valid – absolute references starting with either resolve to CDN-root paths */
    public const PUBLIC_BASES = ['https://cdn.scutoons.com/', 'https://scutoons.b-cdn.net/'];
    /** the extensions content may reference as media (usage scanner) */
    public const MEDIA_EXTENSION = '/\.(jpe?g|png|webp|gif|svg|avif|mp4|webm|mov|m4v|mp3|m4a|aac|ogg|oga|wav|flac|zip|pdf|mscz|glb)$/i';
    /**
     * Map tiles (agents/MAP.md §8.9, research/ASSET-LISTS.md): a file `<set>/<z>/<x>_<y>.<ext>` below a `/map/` folder –
     * `catpope/map/terrain/painted/v2/-1/0_0.webp` belongs to the tile set `catpope/map/terrain/painted/v2`. Thousands
     * of them, never picked one by one: the lists hide them by default, Unused shows a set as one row. TILE_SQL is the
     * same test on `assets.slug` (MariaDB REGEXP) – keep both in step (AssetsTest).
     */
    public const TILE = '#^(.+/map/.+)/-?\d+/\d+_\d+$#';
    public const TILE_SQL = '/map/.+/-?[0-9]+/[0-9]+_[0-9]+$';

    /** the tile set a slug or path belongs to (`catpope/map/terrain/painted/v2`), null when it is no map tile */
    public static function tileSet(string $slugOrPath): ?string
    {
        $slug = (string) preg_replace('/\.[a-z0-9]+$/i', '', $slugOrPath);
        return preg_match(self::TILE, $slug, $m) ? $m[1] : null;
    }

    public static function extension(string $key): string
    {
        return strtolower(pathinfo($key, PATHINFO_EXTENSION));
    }

    public static function kind(string $key): string
    {
        $ext = self::extension($key);
        return match (true) {
            in_array($ext, self::IMAGE, true) => 'image',
            in_array($ext, self::VIDEO, true) => 'video',
            in_array($ext, self::AUDIO, true) => 'audio',
            $ext === 'zip' => 'archive',
            in_array($ext, self::MODEL, true) => 'model',
            in_array($ext, ['pdf', 'mscz', 'musicxml', 'mxl', 'xml', 'json', 'txt'], true) => 'document',
            default => 'other',
        };
    }

    /** the top-level folder (a toon id, characters, scutoons …), null for a file at the root */
    public static function folder(string $key): ?string
    {
        $pos = strpos($key, '/');
        return $pos === false ? null : substr($key, 0, $pos);
    }

    /**
     * @return array{slug: string, marker: ?string, quality: ?string, ext: string, kind: string, rank: int}
     *   marker = the rendition label taken from the name (480p, 1280w, 128k, 2x …), null for a plain file;
     *   rank = how "big" the marker is (height, width, bitrate, density) – the biggest plain-less file is the source
     */
    public static function parse(string $key): array
    {
        $key = StorageKey::normalize($key);
        $ext = self::extension($key);
        $kind = self::kind($key);
        $dir = StorageKey::dirname($key);
        $base = StorageKey::basename($key);
        $stem = $ext === '' ? $base : substr($base, 0, -strlen($ext) - 1);
        $marker = null;
        $quality = null;
        $rank = 0;

        if ($kind === 'video' || $kind === 'image') {
            if (preg_match('/^(.+)\.(\d{3,4})p$/', $stem, $m)) {
                [$stem, $marker, $rank] = [$m[1], $m[2] . 'p', (int) $m[2]];
                $quality = self::VIDEO_QUALITY[(int) $m[2]] ?? null;
            } elseif (preg_match('/^(.+)\.(\d{3,4})w$/', $stem, $m)) {
                [$stem, $marker, $rank] = [$m[1], $m[2] . 'w', (int) $m[2]];
            }
        }
        if ($kind === 'audio' && preg_match('/^(.+)\.(\d{2,3})$/', $stem, $m)) {
            [$stem, $marker, $rank] = [$m[1], $m[2] . 'k', (int) $m[2]];
            $quality = (int) $m[2] <= 96 ? 'low' : ((int) $m[2] <= 160 ? 'medium' : 'high');
        }
        if ($kind === 'image' && $marker === null) {
            if (preg_match('/^(.+)\.poster$/', $stem, $m)) {
                // a video's poster frame (AssetJobs "poster") belongs to the video
                [$stem, $marker, $rank] = [$m[1], 'poster', 0];
            } elseif (preg_match('/^(.+)@([123])x$/', $stem, $m)) {
                [$stem, $marker, $rank] = [$m[1], $m[2] . 'x', (int) $m[2]];
                $quality = self::DENSITY_QUALITY[$marker];
            } elseif (preg_match('#(^|/)([123]x)(/|$)#', $dir, $m)) {
                // images/1x/<rest> and images/3x/<rest>: the density folder is the marker
                $marker = $m[2];
                $rank = (int) $marker[0];
                $quality = self::DENSITY_QUALITY[$marker];
                $dir = (string) preg_replace('#(^|/)[123]x(/|$)#', '$1', $dir, 1);
                $dir = rtrim($dir, '/');
            }
        }
        $slug = ($dir === '' ? '' : $dir . '/') . $stem;
        return ['slug' => $slug, 'marker' => $marker, 'quality' => $quality, 'ext' => $ext, 'kind' => $kind, 'rank' => $rank];
    }

    /**
     * Groups storage keys into assets.
     * @param list<string> $keys
     * @return array<string, array{kind: string, source: string, files: array<string, array{label: string, quality: ?string}>}> slug → group
     */
    public static function group(array $keys): array
    {
        $groups = [];
        foreach ($keys as $key) {
            $parsed = self::parse($key);
            $groups[$parsed['slug']][$key] = $parsed;
        }
        $out = [];
        ksort($groups);
        foreach ($groups as $slug => $files) {
            $source = self::source($files);
            $kinds = array_unique(array_column($files, 'kind'));
            $group = ['kind' => count($kinds) === 1 ? $kinds[0] : $files[$source]['kind'], 'source' => $source, 'files' => []];
            ksort($files);
            foreach ($files as $key => $parsed) {
                $group['files'][$key] = ['label' => self::label($parsed, $key === $source), 'quality' => $parsed['quality']];
            }
            $out[(string) $slug] = $group;
        }
        return $out;
    }

    /**
     * The file a version is "made of": a plain file in a source format (png / jpg / svg / mp4 …, not a WebP derivative)
     * wins, then any plain file, then the biggest marker (1080p, 1920w, 320k, 3x).
     * @param array<string, array{marker: ?string, ext: string, rank: int}> $files
     */
    public static function source(array $files): string
    {
        $best = null;
        $bestScore = -1;
        foreach ($files as $key => $parsed) {
            $score = $parsed['marker'] === null ? ($parsed['ext'] !== 'webp' ? 3_000_000 : 2_000_000) : $parsed['rank'];
            if ($score > $bestScore || ($score === $bestScore && strcmp((string) $key, (string) $best) < 0)) {
                $best = (string) $key;
                $bestScore = $score;
            }
        }
        return (string) $best;
    }

    /** @param array{marker: ?string, ext: string} $parsed */
    public static function label(array $parsed, bool $isSource): string
    {
        if ($parsed['marker'] !== null) return $parsed['marker'];
        return $isSource ? 'original' : ($parsed['ext'] ?: 'file');
    }

    /**
     * A content string → the CDN-root key it points to, relative to $base (the toon folder of the document; '' = the
     * CDN root). `../characters/x.webp` in catpope → characters/x.webp. Full URLs on one of $publicBases are stripped;
     * other URLs, empty and invalid paths → null.
     * @param list<string> $publicBases
     */
    public static function resolve(string $raw, string $base, array $publicBases = []): ?string
    {
        $value = trim($raw);
        if ($value === '' || strlen($value) > 500) return null;
        if (preg_match('#^[a-z]+://#i', $value)) {
            foreach ($publicBases as $public) {
                $public = rtrim($public, '/') . '/';
                if ($public !== '/' && str_starts_with($value, $public)) return self::normalize(substr($value, strlen($public)));
            }
            return null;
        }
        $value = (string) preg_replace('/[?#].*$/', '', $value);
        if (str_starts_with($value, '/')) return self::normalize($value);
        return self::normalize(($base === '' ? '' : $base . '/') . $value);
    }

    /** resolves "." / ".." segments; null when it climbs above the root */
    public static function normalize(string $path): ?string
    {
        $out = [];
        foreach (explode('/', str_replace('\\', '/', $path)) as $segment) {
            if ($segment === '' || $segment === '.') continue;
            if ($segment === '..') {
                if (!$out) return null;
                array_pop($out);
                continue;
            }
            $out[] = $segment;
        }
        return $out ? implode('/', $out) : null;
    }

    /**
     * The string to write into a document for $key, keeping the style of the reference it replaces: relative to the
     * document's toon folder when $old was relative to it, `../`-relative when $old climbed out, root-relative otherwise.
     */
    public static function relativeTo(string $key, string $base, string $old): string
    {
        $old = trim($old);
        if (preg_match('#^[a-z]+://#i', $old)) {
            $prefix = substr($old, 0, (int) strpos($old, '/', (int) strpos($old, '://') + 3) + 1);
            return $prefix . $key;
        }
        if (str_starts_with($old, '/')) return '/' . $key;
        if ($base === '') return $key;
        if (str_starts_with($key, $base . '/')) return substr($key, strlen($base) + 1);
        return '../' . $key;
    }
}
