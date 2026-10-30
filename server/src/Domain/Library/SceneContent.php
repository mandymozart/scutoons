<?php
declare(strict_types=1);

namespace Scutoons\Domain\Library;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Assets\AssetRegistry;
use Scutoons\Domain\Catalog\CatalogStore;

/**
 * What a deck's scene needs (#195 "sync as a deck feature", #193 sets by id): `GET /api/library/scene-content?sets=&characters=`
 * – each set's 3D variant per quality level and the portrait mood sprites of each character, CDN paths (immutable: a
 * changed file gets a new name, so the path is the version) with their sizes from the asset registry. The deck downloads
 * what its cache lacks (deck/stage sync.rs) and plays offline from the cache.
 *
 * - **Sets** (agents/DECISIONS.md 2026-10-30: place → set → assets) are the catalog's home scenes for now (home scenes
 *   become sets) – their `model: { src, variants? }` = the 3D variant, a `.glb` per quality (`src` = medium; a level
 *   without its own variant gets `src`), answered as `model: { low, medium, high }`; a set without one answers `model: {}`.
 * - **Characters**: a registry id (`catpope`), an old VN id (`plf` → Library::LEGACY_CHARACTER_IDS) or `<character>-<persona>`
 *   (`catpope-biker`, the deck's stand-ins) – that persona's (else `regular`'s) default look, its `portrait` per mood.
 * - Public like the catalog and the registry; a deck token (#180, DeckTokenScope lets GETs through) changes nothing yet –
 *   member-only sets would read the token's member here.
 */
final class SceneContent
{
    public const MAX_IDS = 32;
    public const ID = '#^[a-z0-9][a-z0-9_-]{0,63}$#';

    public function __construct(
        private readonly Library $library,
        private readonly CatalogStore $catalog,
        private readonly AssetRegistry $registry,
    ) {
    }

    /**
     * @param list<string> $sets
     * @param list<string> $characters
     * @return array{sets: array<string, mixed>, characters: array<string, mixed>, missing: array{sets: list<string>, characters: list<string>}}
     */
    public function of(array $sets, array $characters): array
    {
        $registry = [];
        if ($characters !== []) {
            try {
                $registry = json_decode($this->library->characters(false)->body, true);
            } catch (ApiError) {
                $registry = []; // no registry stored: every character is missing
            }
        }
        $answer = self::compose($sets, fn(string $id) => $this->catalog->get('home-scenes', $id), $characters, is_array($registry) ? $registry : []);
        $sizes = $this->registry->available() ? $this->registry->sizesOf(self::paths($answer)) : [];
        return self::withSizes($answer, $sizes);
    }

    /**
     * The answer without sizes (pure – SceneContentTest).
     *
     * @param list<string> $sets
     * @param callable(string): mixed $set a set's catalog entry by id (null = none)
     * @param list<string> $characters
     * @param array<mixed> $registry the character registry ({ characters: [...] })
     * @return array{sets: array<string, mixed>, characters: array<string, mixed>, missing: array{sets: list<string>, characters: list<string>}}
     */
    public static function compose(array $sets, callable $set, array $characters, array $registry): array
    {
        $out = ['sets' => [], 'characters' => [], 'missing' => ['sets' => [], 'characters' => []]];
        foreach (array_values(array_unique($sets)) as $id) {
            $entry = $set($id);
            if (!is_array($entry)) {
                $out['missing']['sets'][] = $id;
                continue;
            }
            $model3d = [];
            $model = $entry['model'] ?? null;
            if (is_array($model) && is_string($model['src'] ?? null) && $model['src'] !== '') {
                $variants = is_array($model['variants'] ?? null) ? $model['variants'] : [];
                foreach (ChapterFiles::LEVELS as $level) {
                    $file = $variants[$level] ?? null;
                    $model3d[$level] = ['path' => is_string($file) && $file !== '' ? $file : $model['src'], 'size' => null];
                }
            }
            $out['sets'][$id] = ['name' => $entry['name'] ?? null, 'model' => (object) $model3d];
        }
        $byId = [];
        foreach (is_array($registry['characters'] ?? null) ? $registry['characters'] : [] as $c) {
            if (is_array($c) && is_string($c['id'] ?? null)) $byId[$c['id']] = $c;
        }
        foreach (array_values(array_unique($characters)) as $id) {
            $found = self::resolve($id, $byId);
            if ($found === null) {
                $out['missing']['characters'][] = $id;
                continue;
            }
            [$character, $persona, $look] = $found;
            $portraits = [];
            foreach (is_array($look['images'] ?? null) ? $look['images'] : [] as $mood => $views) {
                $path = is_array($views) ? ($views['portrait'] ?? null) : null;
                if (is_string($path) && $path !== '') $portraits[(string) $mood] = ['path' => $path, 'size' => null];
            }
            ksort($portraits);
            $out['characters'][$id] = [
                'character' => $character,
                'persona' => $persona,
                'look' => is_string($look['id'] ?? null) ? $look['id'] : null,
                'portraits' => (object) $portraits,
            ];
        }
        return $out;
    }

    /**
     * A deck's character name → [registry id, persona id, its default look]: the id itself, an old VN id, or
     * `<character>-<persona>` (the first split whose left part is a character with that persona)
     *
     * @param array<string, array<mixed>> $byId
     * @return array{0: string, 1: string, 2: array<mixed>}|null
     */
    private static function resolve(string $id, array $byId): ?array
    {
        $candidates = [[$id, 'regular']];
        if (isset(Library::LEGACY_CHARACTER_IDS[$id])) $candidates[] = [Library::LEGACY_CHARACTER_IDS[$id], 'regular'];
        for ($i = strpos($id, '-'); $i !== false; $i = strpos($id, '-', $i + 1)) {
            $candidates[] = [substr($id, 0, $i), substr($id, $i + 1)];
        }
        foreach ($candidates as [$character, $persona]) {
            $c = $byId[$character] ?? null;
            if ($c === null) continue;
            foreach (is_array($c['personas'] ?? null) ? $c['personas'] : [] as $p) {
                if (!is_array($p) || ($p['id'] ?? null) !== $persona) continue;
                $looks = array_values(array_filter(is_array($p['looks'] ?? null) ? $p['looks'] : [], 'is_array'));
                $default = array_values(array_filter($looks, fn(array $l) => ($l['default'] ?? false) === true))[0] ?? $looks[0] ?? [];
                return [$character, $persona, $default];
            }
        }
        return null;
    }

    /**
     * every file path of an answer
     * @param array{sets: array<string, mixed>, characters: array<string, mixed>} $answer
     * @return list<string>
     */
    public static function paths(array $answer): array
    {
        $paths = [];
        foreach ($answer['sets'] as $s) foreach ((array) $s['model'] as $f) $paths[] = $f['path'];
        foreach ($answer['characters'] as $c) foreach ((array) $c['portraits'] as $f) $paths[] = $f['path'];
        return array_values(array_unique($paths));
    }

    /**
     * @param array{sets: array<string, mixed>, characters: array<string, mixed>, missing: array{sets: list<string>, characters: list<string>}} $answer
     * @param array<string, int> $sizes path → bytes (unknown paths keep `size: null`)
     * @return array{sets: array<string, mixed>, characters: array<string, mixed>, missing: array{sets: list<string>, characters: list<string>}}
     */
    public static function withSizes(array $answer, array $sizes): array
    {
        $fill = function (object $files) use ($sizes): object {
            $out = [];
            foreach ((array) $files as $k => $f) $out[$k] = ['path' => $f['path'], 'size' => $sizes[$f['path']] ?? null];
            return (object) $out;
        };
        foreach ($answer['sets'] as $id => $s) $answer['sets'][$id]['model'] = $fill($s['model']);
        foreach ($answer['characters'] as $id => $c) $answer['characters'][$id]['portraits'] = $fill($c['portraits']);
        return $answer;
    }
}
