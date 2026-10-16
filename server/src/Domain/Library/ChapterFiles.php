<?php
declare(strict_types=1);

namespace Scutoons\Domain\Library;

use Scutoons\Domain\Assets\AssetPaths;
use Scutoons\Domain\Assets\AssetRegistry;
use Scutoons\Domain\Catalog\CatalogStore;
use Scutoons\Infrastructure\Config\Settings;
use Throwable;

/**
 * A chapter's media for "save to device" (PWA phase 3, agents/research/PWA.md §2 "Phase 3", DATA.md §6, FEATURES 10.74):
 * `GET /api/library/toons/{toon}/chapters/{n}/files?locale=` – one request, every file the reader needs offline, with its
 * size from the asset registry and the quality levels that use it, so the viewer shows "about 18 MB" before a byte moves
 * and downloads only its own level.
 *
 * - The chapter as the library answers it (scenes inlined; the same visibility gates: coming soon / not public → 403,
 *   never a draft): every media string (slice images, videos, key visual, thumbnail, music, samples …). A media object
 *   with quality variants – `{ src | path, variants }` (videos, audio) or `{ image, imageVariants }` (slices) – yields the
 *   file each level plays (@scutoons/content `pickQualityVariant`: the level's variant, else the nearest lower, else
 *   the default), marked with those levels and the media's `group` (offline, the saved level's file answers for the
 *   group's other files); everything else is used at every level.
 * - The characters the chapter's VN puts on stage (`character` + `persona` refs; not `featuring` – the reader doesn't
 *   show it): those personas' portrait and avatar views (+ the regular persona, the fallback anchor) and the
 *   character's image – no `full` view (unused), no A-pose, no 3D model.
 * - The catalog items it names (`item` fields, `related_items`): their icons and their variants' icons.
 * Only images, audio and video; paths from the CDN root (the viewer builds `<cdn>/<path>`, like toonUrl / asset).
 */
final class ChapterFiles
{
    public const LEVELS = ['low', 'medium', 'high'];
    private const KINDS = ['image', 'video', 'audio'];
    /** character views a reader shows (`full` is unused, the A-pose belongs to the avatar page) */
    private const SKIP_CHARACTER_KEYS = ['full', 'apose', 'model', 'models'];

    private int $groupCount = 0;

    public function __construct(
        private readonly Library $library,
        private readonly AssetRegistry $registry,
        private readonly CatalogStore $catalog,
        private readonly Settings $settings,
    ) {
    }

    /**
     * @return array{toon: string, chapter: int, locale: ?string, version: string, gated: bool,
     *   files: list<array{path: string, size: ?int, kind: string, levels?: list<string>, group?: int}>}
     */
    public function of(string $toon, int $number, ?string $locale): array
    {
        // the library's own gates (403 coming_soon / not_public, 404) – never draft mode
        $answer = $this->library->chapter($toon, $number, $locale, false);
        $this->groupCount = 0;
        $chapter = json_decode($answer->body, true);
        $chapter = is_array($chapter) ? $chapter : [];

        /** @var array<string, array<string, true>> $files path → levels (all three = every level) */
        $files = [];
        /** @var array<string, int> $groups path → its quality group (the first it is in) */
        $groups = [];
        $add = function (string $raw, string $base, array $levels, ?int $group = null) use (&$files, &$groups): void {
            $path = AssetPaths::resolve($raw, $base, $this->publicBases());
            if ($path === null || !in_array(AssetPaths::kind($path), self::KINDS, true)) return;
            foreach ($levels as $level) $files[$path][$level] = true;
            if ($group !== null) $groups[$path] ??= $group;
        };

        $this->walk($chapter, $toon, $add);

        // the characters on stage (VN refs: character + persona): those personas' reader views, the regular one too
        // (the fallback anchor)
        $personas = self::characterRefs($chapter);
        if ($personas !== []) {
            try {
                $cast = json_decode($this->library->cast($toon, $number, false)->body, true);
                foreach (is_array($cast['registry']['characters'] ?? null) ? $cast['registry']['characters'] : [] as $character) {
                    if (!is_array($character) || !isset($personas[(string) ($character['id'] ?? '')])) continue;
                    $wanted = $personas[(string) $character['id']];
                    if (is_string($character['image'] ?? null)) $add($character['image'], '', self::LEVELS);
                    foreach (is_array($character['personas'] ?? null) ? $character['personas'] : [] as $persona) {
                        if (is_array($persona) && isset($wanted[(string) ($persona['id'] ?? '')])) $this->walk($persona, '', $add, self::SKIP_CHARACTER_KEYS);
                    }
                }
            } catch (Throwable) {
                // no cast / registry: the chapter's own files still save
            }
        }

        // the catalog items it names: their icons (CDN root, like the viewer's asset())
        foreach (array_keys(self::itemIds($chapter)) as $id) {
            $item = $this->catalog->get('items', (string) $id);
            if (!is_array($item)) continue;
            if (is_string($item['icon'] ?? null)) $add($item['icon'], '', self::LEVELS);
            foreach (is_array($item['variants'] ?? null) ? $item['variants'] : [] as $variant) {
                if (is_array($variant) && is_string($variant['icon'] ?? null)) $add($variant['icon'], '', self::LEVELS);
            }
        }

        $sizes = $this->registry->available() ? $this->registry->sizesOf(array_map('strval', array_keys($files))) : [];
        $out = [];
        foreach ($files as $path => $levels) {
            $path = (string) $path;
            $entry = ['path' => $path, 'size' => $sizes[$path] ?? null, 'kind' => AssetPaths::kind($path)];
            $used = array_values(array_filter(self::LEVELS, fn(string $l) => isset($levels[$l])));
            if (count($used) < count(self::LEVELS)) {
                $entry['levels'] = $used;
                // the files of one media at its levels share a group: offline, the saved one stands in for the others
                if (isset($groups[$path])) $entry['group'] = $groups[$path];
            }
            $out[] = $entry;
        }
        usort($out, fn(array $a, array $b) => strcmp($a['path'], $b['path']));

        return [
            'toon' => $toon,
            'chapter' => $number,
            'locale' => $answer->locale,
            // the chapter answer's ETag: a saved chapter whose version changed can be refreshed
            'version' => trim($answer->etag, '"'),
            // members only unless the chapter is a free preview (the viewer's MembershipBlocker)
            'gated' => !($chapter['isPreview'] ?? false),
            'files' => $out,
        ];
    }

    /**
     * Every media string below `$node`; quality groups yield one file per level
     * @param array<mixed> $node
     * @param callable(string, string, list<string>, ?int=): void $add
     * @param list<string> $skip keys not walked
     */
    private function walk(array $node, string $base, callable $add, array $skip = []): void
    {
        $done = [];
        // { src | path, variants } – a video or a track / sample with its quality files
        $default = is_string($node['src'] ?? null) ? 'src' : (is_string($node['path'] ?? null) ? 'path' : null);
        if ($default !== null && is_array($node['variants'] ?? null)) {
            $this->group((string) $node[$default], $node['variants'], $base, $add);
            $done = [$default, 'variants'];
        }
        // { image, imageVariants } – a slice image with its 2x / 3x
        if (is_string($node['image'] ?? null) && is_array($node['imageVariants'] ?? null)) {
            $this->group((string) $node['image'], $node['imageVariants'], $base, $add);
            $done = [...$done, 'image', 'imageVariants'];
        }
        foreach ($node as $key => $value) {
            if (in_array($key, $done, true) || in_array($key, $skip, true)) continue;
            if (is_array($value)) $this->walk($value, $base, $add, $skip);
            elseif (is_string($value) && preg_match(AssetPaths::MEDIA_EXTENSION, trim((string) preg_replace('/[?#].*$/', '', $value)))) {
                $add($value, $base, self::LEVELS);
            }
        }
    }

    /**
     * @param array<mixed> $variants
     * @param callable(string, string, list<string>, ?int=): void $add
     */
    private function group(string $default, array $variants, string $base, callable $add): void
    {
        $group = $this->groupCount++;
        foreach (self::LEVELS as $level) $add(self::pick($default, $variants, $level), $base, [$level], $group);
    }

    /**
     * @scutoons/content pickQualityVariant: the level's variant, else the nearest lower one, else the default
     * @param array<mixed> $variants
     */
    public static function pick(string $default, array $variants, string $level): string
    {
        for ($i = (int) array_search($level, self::LEVELS, true); $i >= 0; $i--) {
            $file = $variants[self::LEVELS[$i]] ?? null;
            if (is_string($file) && $file !== '') return $file;
        }
        return $default;
    }

    /**
     * The catalog item ids a chapter names: placed items and events (`item` fields) and its `related_items`
     * @param array<mixed> $node
     * @param array<string, true> $ids
     * @return array<string, true>
     */
    private static function itemIds(array $node, array &$ids = []): array
    {
        foreach ($node as $key => $value) {
            if ($key === 'item' && is_string($value) && $value !== '') $ids[$value] = true;
            elseif ($key === 'related_items' && is_array($value)) {
                foreach ($value as $id) if (is_string($id) && $id !== '') $ids[$id] = true;
            } elseif (is_array($value)) self::itemIds($value, $ids);
        }
        return $ids;
    }

    /**
     * character id → persona ids a chapter's VN refs put on stage (legacy ids mapped; `regular` always)
     * @param array<mixed> $node
     * @param array<string, array<string, true>> $refs
     * @return array<string, array<string, true>>
     */
    private static function characterRefs(array $node, array &$refs = []): array
    {
        if (is_string($node['character'] ?? null) && $node['character'] !== '') {
            $id = Library::LEGACY_CHARACTER_IDS[$node['character']] ?? $node['character'];
            $refs[$id]['regular'] = true;
            if (is_string($node['persona'] ?? null) && $node['persona'] !== '') $refs[$id][$node['persona']] = true;
        }
        foreach ($node as $value) {
            if (is_array($value)) self::characterRefs($value, $refs);
        }
        return $refs;
    }

    /** @return list<string> */
    private function publicBases(): array
    {
        return array_values(array_filter(array_unique([$this->settings->get('CDN_PUBLIC_URL'), $this->settings->get('CDN_URL'), ...AssetPaths::PUBLIC_BASES])));
    }
}
