<?php
declare(strict_types=1);

namespace Scutoons\Domain\Library;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Assets\AssetRegistry;
use Scutoons\Domain\Catalog\CatalogStore;
use Scutoons\Domain\Catalog\CatalogValidator;
use Scutoons\Generated\ContentTables;
use stdClass;

/**
 * What a deck's scene needs (#195 "sync as a deck feature", #193 sets by id): `GET /api/library/scene-content?sets=&characters=`
 * – each set's 3D variant per quality level and the portrait mood sprites of each character, CDN paths (immutable: a
 * changed file gets a new name, so the path is the version) with their sizes from the asset registry. The deck downloads
 * what its cache lacks (deck/native sync.rs) and plays offline from the cache.
 *
 * - **Sets** (agents/DECISIONS.md 2026-10-30: place → set → assets) are the catalog's home scenes for now (home scenes
 *   become sets) – their `model: { src, variants? }` = the 3D variant, a `.glb` per quality (`src` = medium; a level
 *   without its own variant gets `src`), answered as `model: { low, medium, high }`; a set without one answers `model: {}`.
 * - **Characters**: a registry id (`catpope`), an old VN id (`plf` → its character and persona, ContentTables::LEGACY_CHARACTER_IDS) or `<character>-<persona>`
 *   (`catpope-biker`, the deck's stand-ins) – that persona's (else `regular`'s) default look, its `portrait` per mood.
 * - **Rigs** (#258, agents/research/INSTRUMENTS.md §6): each character's `rig` – the persona's own
 *   (`character.<id>.<persona>`), else the character's (`character.<id>`, persona variants inside), else null – and per
 *   `items=` id its rig (`item.<id>`, a scene object's sound, #210). A rig is `{ id, version, data, instruments, presets }`:
 *   `data` = deck-space's `Rig`, `instruments` / `presets` = the ids it needs, presets **with their ancestors** (a lineage
 *   resolves offline). The top-level `instruments` / `presets` maps hold each one once as `{ id, version, data }`;
 *   `version` = a hash of the stored data, so a deck's cache skips what it has. `{ rig: data, instrument: […], preset: […] }`
 *   is a deck-space `rig::Bundle` (its the_servers_scene_content_reads_as_bundles test reads this answer). A code rig (`policy`,
 *   #408 / #414) also names its policy module, `modules: [id]`; the top-level `modules` map (there only when a rig has one)
 *   holds its descriptor as `data` (`Bundle.module`) – never its sources.
 * - `&presets=<ids>` adds factory presets to the top-level map with their ancestors; an unknown id lands in
 *   `missing.presets`. Everything is public like the catalog and the registry (readers save nothing, #302).
 */
final class SceneContent
{
    public const MAX_IDS = 32;
    public const ID = '#^[a-z0-9][a-z0-9_-]{0,63}$#';
    /** the rig kinds as scene-content names them → their catalog kinds */
    private const RIG_KINDS = ['instruments' => 'instruments', 'presets' => 'presets', 'rigs' => 'rigs', 'modules' => 'modules'];

    public function __construct(
        private readonly Library $library,
        private readonly CatalogStore $catalog,
        private readonly AssetRegistry $registry,
    ) {
    }

    /**
     * @param list<string> $sets
     * @param list<string> $characters
     * @param list<string> $items
     * @param list<string> $presets preset ids to add (with their ancestors)
     * @return array<string, mixed> { sets, characters, items, instruments, presets, missing: { sets, characters, items, presets } }
     */
    public function of(array $sets, array $characters, array $items = [], array $presets = []): array
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
        return self::withRigs(self::withSizes($answer, $sizes), $items, fn(string $kind, string $id) => $this->stored($kind, $id), $presets);
    }

    /**
     * a rig kind's stored data (the catalog row's JSON), null when there is none – or no table yet (before the migration)
     */
    private function stored(string $kind, string $id): ?string
    {
        try {
            $row = $this->catalog->row(self::RIG_KINDS[$kind], $id);
        } catch (\Throwable) {
            return null;
        }
        return $row['data'] ?? null;
    }

    /**
     * The answer with the rigs (pure – SceneContentRigsTest): `characters.<id>.rig`, `items`, `instruments`, `presets`
     * (+ the asked `$presets` with their ancestors, #270), `missing.items`, `missing.presets`.
     *
     * @param array{sets: array<string, mixed>, characters: array<string, mixed>, missing: array{sets: list<string>, characters: list<string>}} $answer
     * @param list<string> $items
     * @param callable(string, string): ?string $stored (kind instruments | presets | rigs, id) → the stored JSON
     * @param list<string> $presets
     * @return array<string, mixed>
     */
    public static function withRigs(array $answer, array $items, callable $stored, array $presets = []): array
    {
        $out = ['instruments' => [], 'presets' => []];
        // a preset and its ancestors, root first (a loop or a missing parent stops the walk – the catalog refuses both);
        // their instruments come along. [] when the preset itself is unknown
        $lineage = function (string $id, array &$instruments) use ($stored, &$out): array {
            $chain = [];
            for ($p = $id; is_string($p) && !in_array($p, $chain, true); $p = $parent) {
                $preset = self::entry($stored, 'presets', $p, $out);
                if ($preset === null) break;
                $chain[] = $p;
                $parent = $preset->parent ?? null;
                if (is_string($preset->instrument ?? null)) $instruments[] = $preset->instrument;
            }
            return array_reverse($chain);
        };
        $rig = function (string $subject) use ($stored, &$out, $lineage): ?array {
            $json = $stored('rigs', $subject);
            $data = $json !== null ? CatalogStore::reader($json, false) : null;
            if (!$data instanceof stdClass) return null;
            $instruments = [];
            $presets = [];
            $tracks = is_array($data->track ?? null) ? $data->track : [];
            foreach ($tracks as $t) {
                if ($t instanceof stdClass && is_string($t->instrument ?? null)) $instruments[] = $t->instrument;
            }
            // a code rig's policy module (#414): its descriptor, once, in the top-level `modules`; the presets it chooses from
            // come along with the ones the rig names
            $policy = $data->policy ?? null;
            $module = $policy instanceof stdClass ? ($policy->module ?? null) : null;
            $descriptor = is_string($module) ? self::module($stored, $module, $out) : null;
            $modules = $descriptor !== null ? [$module] : [];
            $named = [...CatalogValidator::rigPresets(json_decode((string) $json, true)), ...($descriptor !== null ? CatalogValidator::policyPresets($descriptor) : [])];
            foreach ($named as $id) {
                $presets = [...$presets, ...$lineage($id, $instruments)];
            }
            $instruments = array_values(array_unique($instruments));
            foreach ($instruments as $i) self::entry($stored, 'instruments', $i, $out);
            $have = fn(string $kind) => fn(string $id) => isset($out[$kind][$id]);
            return [
                'id' => $subject,
                'version' => self::version((string) $json),
                'data' => $data,
                'instruments' => array_values(array_filter($instruments, $have('instruments'))),
                'presets' => array_values(array_filter(array_values(array_unique($presets)), $have('presets'))),
            ] + ($modules === [] ? [] : ['modules' => $modules]);
        };
        foreach ($answer['characters'] as $id => $c) {
            $persona = is_string($c['persona'] ?? null) ? $c['persona'] : 'regular';
            $answer['characters'][$id]['rig'] = $rig("character.{$c['character']}.$persona") ?? $rig("character.{$c['character']}");
        }
        $answer['items'] = [];
        $answer['missing']['items'] = [];
        foreach (array_values(array_unique($items)) as $id) {
            $found = $rig("item.$id");
            if ($found === null) $answer['missing']['items'][] = $id;
            else $answer['items'][$id] = ['rig' => $found];
        }
        // #270: the asked presets (a member's own derivatives, or factory ones) with their ancestors
        $answer['missing']['presets'] = [];
        foreach (array_values(array_unique($presets)) as $id) {
            $instruments = [];
            if ($lineage($id, $instruments) === []) $answer['missing']['presets'][] = $id;
            foreach (array_unique($instruments) as $i) self::entry($stored, 'instruments', $i, $out);
        }
        ksort($out['instruments']);
        ksort($out['presets']);
        if (isset($out['modules'])) ksort($out['modules']);
        return [...$answer, ...$out];
    }

    /**
     * A code rig's policy module (#414, research/RIG-MODULE-GROUPS.md decision 5) as the answer's `modules` entry: the
     * descriptor the compile service stored in the `module:<id>` document (not its sources), `version` = its hash. Null
     * when there is none – no such module, not compiled yet, or a descriptor under another name (the deck's bundle then
     * misses it, as it misses an instrument that is gone).
     * @param callable(string, string): ?string $stored
     * @param array<string, array<string, mixed>> $out
     */
    private static function module(callable $stored, string $id, array &$out): ?stdClass
    {
        if (isset($out['modules'][$id])) return $out['modules'][$id]['data'];
        $json = $stored('modules', $id);
        $doc = $json !== null ? CatalogStore::reader($json, false) : null;
        $descriptor = $doc instanceof stdClass ? ($doc->descriptor ?? null) : null;
        if (!$descriptor instanceof stdClass || ($descriptor->name ?? null) !== $id) return null;
        $out['modules'][$id] = ['id' => $id, 'version' => self::version((string) json_encode($descriptor, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_PRESERVE_ZERO_FRACTION)), 'data' => $descriptor];
        return $descriptor;
    }

    /**
     * an instrument / preset of the answer (added to $out once), decoded with objects
     * @param callable(string, string): ?string $stored
     * @param array<string, array<string, mixed>> $out
     */
    private static function entry(callable $stored, string $kind, string $id, array &$out): ?stdClass
    {
        if (isset($out[$kind][$id])) return $out[$kind][$id]['data'];
        $json = $stored($kind, $id);
        $data = $json !== null ? CatalogStore::reader($json, false) : null;
        if (!$data instanceof stdClass) return null;
        $out[$kind][$id] = ['id' => $id, 'version' => self::version($json), 'data' => $data];
        return $data;
    }

    /**
     * The answer as JSON: maps as objects (an empty one too), 1.0 stays 1.0 (deck-space's node params keep floats floats)
     * @param array<string, mixed> $content of()
     */
    public static function json(array $content): string
    {
        return (string) json_encode([
            'sets' => (object) $content['sets'],
            'characters' => (object) $content['characters'],
            'items' => (object) ($content['items'] ?? []),
            'instruments' => (object) ($content['instruments'] ?? []),
            'presets' => (object) ($content['presets'] ?? []),
            ...(($content['modules'] ?? []) === [] ? [] : ['modules' => (object) $content['modules']]),
            'missing' => $content['missing'],
        ], JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_PRESERVE_ZERO_FRACTION);
    }

    /** a stored entry's version for the decks' cache: its data's hash (any change = a new version) */
    public static function version(string $json): string
    {
        return substr(hash('sha256', $json), 0, 16);
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
        $legacy = ContentTables::LEGACY_CHARACTER_IDS[$id] ?? null;
        // the persona its art went to (#477: the one table, the same answer as the reader's), else its character's regular
        if ($legacy !== null) array_push($candidates, [$legacy['character'], $legacy['persona']], [$legacy['character'], 'regular']);
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
