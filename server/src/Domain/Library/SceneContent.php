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
 *   become sets) – their media per rung of the render ladder (research/SCENES.md §9.4, #512), each `{ src, variants? }`
 *   answered per quality as `{ low, medium, high }` (a level without its own variant gets `src`; none = `{}`): `model` =
 *   the 3D variant, a `.glb` (rung 1; `src` = medium), `video` = its loop (rung 2), `still` = its `image` (rung 3, the
 *   video's poster; `src` = 1080p).
 * - **Locations** (#512): a place's `set` (`place:<id>`, owner 2026-11-10 [live]: a location is exactly one set) as
 *   `locations.<id>.set` (null: it names none), its set answered in `sets` beside the asked ones; a place with no published
 *   document is in `missing.locations`. Which set a scene plays is the deck core's (scene_resolve.rs), not this answer's.
 * - **Characters**: a registry id (`catpope`), an old VN id (`plf` → its character and persona, ContentTables::LEGACY_CHARACTER_IDS) or `<character>-<persona>`
 *   (`catpope-biker`, the deck's stand-ins) – that persona's (else `regular`'s) default look, its `portrait` per mood and
 *   (#495, a stage's card) its `full` view per mood; the look's `form` (`card` / `look`, null = the stage decides) and its
 *   3D `model` (the look's, else the persona's, `{ path, size }`; null = none – the deck draws its default character).
 * - **Items** (#495): `items.<id>.picture` – the catalog item's picture (its `icon`), what a stage shows as the item's card;
 *   null when it has none. An item the catalog and the rigs both do not know is in `missing.items`.
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
    /** a set's media as scene-content names them (the render ladder's rungs 1–3) → the home scene's fields */
    private const SET_MEDIA = ['model' => 'model', 'video' => 'video', 'still' => 'image'];
    /** a look's form words (deck-space cast.rs `Form`) */
    private const FORMS = ['card', 'look'];

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
     * @param list<string> $locations place ids whose set to answer (#512)
     * @return array<string, mixed> { sets, locations, characters, items, instruments, presets, missing: { sets, locations, characters, items, presets } }
     */
    public function of(array $sets, array $characters, array $items = [], array $presets = [], array $locations = []): array
    {
        $places = $locations === [] ? [] : $this->library->places($locations);
        $registry = [];
        if ($characters !== []) {
            try {
                $registry = json_decode($this->library->characters(false)->body, true);
            } catch (ApiError) {
                $registry = []; // no registry stored: every character is missing
            }
        }
        $answer = self::compose($sets, fn(string $id) => $this->catalog->get('home-scenes', $id), $characters, is_array($registry) ? $registry : [], $locations, fn(string $id) => $places[$id] ?? null);
        $answer = self::withRigs($answer, $items, fn(string $kind, string $id) => $this->stored($kind, $id), $presets, fn(string $subject) => $this->rigPresets($subject));
        $answer = self::withItemPictures($answer, $items, fn(string $id) => $this->item($id));
        $sizes = $this->registry->available() ? $this->registry->sizesOf(self::paths($answer)) : [];
        return self::withSizes($answer, $sizes);
    }

    /**
     * The asked locations' sets (#512): `locations.<id>.set` = the place's `set` (a home scene id), null when it names
     * none; a place without a document is in `missing`. Stored configuration only: the scene's set is the deck core's.
     *
     * @param list<string> $locations
     * @param callable(string): mixed $place a place's document by id (null = none)
     * @return array{locations: array<string, array{set: ?string}>, missing: list<string>}
     */
    private static function locations(array $locations, callable $place): array
    {
        $out = ['locations' => [], 'missing' => []];
        foreach (array_values(array_unique($locations)) as $id) {
            $doc = $place($id);
            $doc = $doc instanceof stdClass ? (array) $doc : $doc;
            if (!is_array($doc)) {
                $out['missing'][] = $id;
                continue;
            }
            $set = $doc['set'] ?? null;
            $out['locations'][$id] = ['set' => is_string($set) && $set !== '' ? $set : null];
        }
        return $out;
    }

    /** a catalog item's entry, null when there is none (or no table yet) */
    private function item(string $id): mixed
    {
        try {
            return $this->catalog->get('items', $id);
        } catch (\Throwable) {
            return null;
        }
    }

    /**
     * The answer with each asked item's picture (pure – SceneContentTest, #495): `items.<id>.picture` = its catalog
     * entry's `icon` as a file, null without one; an item with a picture but no rig is answered too (`rig` null) and leaves
     * `missing.items`.
     *
     * @param array<string, mixed> $answer withRigs()
     * @param list<string> $items
     * @param callable(string): mixed $item a catalog item's entry by id (null = none)
     * @return array<string, mixed>
     */
    public static function withItemPictures(array $answer, array $items, callable $item): array
    {
        foreach (array_values(array_unique($items)) as $id) {
            $entry = $item($id);
            $entry = $entry instanceof stdClass ? (array) $entry : $entry;
            $icon = is_array($entry) ? ($entry['icon'] ?? null) : null;
            $picture = is_string($icon) && $icon !== '' ? ['path' => $icon, 'size' => null] : null;
            if (isset($answer['items'][$id])) {
                $answer['items'][$id]['picture'] = $picture;
            } elseif ($picture !== null) {
                $answer['items'][$id] = ['rig' => null, 'picture' => $picture];
                $answer['missing']['items'] = array_values(array_diff($answer['missing']['items'] ?? [], [$id]));
            }
        }
        return $answer;
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
     * a rig's presets in the catalog (`rig-preset:<id>` published, deck-space RigPreset), by id; none – or no table yet
     * @return list<stdClass>
     */
    private function rigPresets(string $subject): array
    {
        try {
            $rows = $this->catalog->rows('rig-presets');
        } catch (\Throwable) {
            return [];
        }
        $out = [];
        foreach ($rows as $row) {
            $data = CatalogStore::reader($row['data'], false);
            if ($data instanceof stdClass && ($data->rig ?? null) === $subject) $out[] = $data;
        }
        return $out;
    }

    /**
     * The answer with the rigs (pure – SceneContentRigsTest): `characters.<id>.rig`, `items`, `instruments`, `presets`
     * (+ the asked `$presets` with their ancestors, #270), `missing.items`, `missing.presets`. A rig with presets of its own
     * (#677) carries them as `rig_presets` (the documents), and its `version` covers them.
     *
     * @param array{sets: array<string, mixed>, locations?: array<string, array{set: ?string}>, characters: array<string, mixed>, missing: array{sets: list<string>, locations?: list<string>, characters: list<string>}} $answer
     * @param list<string> $items
     * @param callable(string, string): ?string $stored (kind instruments | presets | rigs, id) → the stored JSON
     * @param list<string> $presets
     * @param ?callable(string): list<stdClass> $rigPresets a rig's subject → its presets' documents
     * @return array<string, mixed>
     */
    public static function withRigs(array $answer, array $items, callable $stored, array $presets = [], ?callable $rigPresets = null): array
    {
        $rigPresets ??= fn(string $subject): array => [];
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
        $rig = function (string $subject) use ($stored, &$out, $lineage, $rigPresets): ?array {
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
            $own = $rigPresets($subject);
            usort($own, fn(stdClass $a, stdClass $b) => strcmp((string) ($a->id ?? ''), (string) ($b->id ?? '')));
            return [
                'id' => $subject,
                'version' => self::version($own === [] ? (string) $json : $json . json_encode($own, CatalogStore::JSON_FLAGS)),
                'data' => $data,
                'instruments' => array_values(array_filter($instruments, $have('instruments'))),
                'presets' => array_values(array_filter(array_values(array_unique($presets)), $have('presets'))),
            ] + ($modules === [] ? [] : ['modules' => $modules]) + ($own === [] ? [] : ['rig_presets' => $own]);
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
            'locations' => (object) ($content['locations'] ?? []),
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
     * @param list<string> $locations place ids whose set to answer (#512): each one's set joins `$sets`
     * @param ?callable(string): mixed $place a place's document by id (null = none)
     * @return array{sets: array<string, mixed>, locations: array<string, array{set: ?string}>, characters: array<string, mixed>, missing: array{sets: list<string>, locations: list<string>, characters: list<string>}}
     */
    public static function compose(array $sets, callable $set, array $characters, array $registry, array $locations = [], ?callable $place = null): array
    {
        $at = self::locations($locations, $place ?? fn() => null);
        $sets = [...$sets, ...array_values(array_filter(array_column($at['locations'], 'set'), 'is_string'))];
        $out = ['sets' => [], 'locations' => $at['locations'], 'characters' => [], 'missing' => ['sets' => [], 'locations' => $at['missing'], 'characters' => []]];
        foreach (array_values(array_unique($sets)) as $id) {
            $entry = $set($id);
            if (!is_array($entry)) {
                $out['missing']['sets'][] = $id;
                continue;
            }
            // per rung (#512): the 3D room, its video loop, its still – each a file per quality level
            $out['sets'][$id] = ['name' => $entry['name'] ?? null] + array_map(fn(string $field) => self::levels($entry[$field] ?? null), self::SET_MEDIA);
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
            [$character, $persona, $look, $personaModel] = $found;
            // per view: mood → file (the heads' portraits; #495 a stage's card shows the full view)
            $views = ['portrait' => [], 'full' => []];
            foreach (is_array($look['images'] ?? null) ? $look['images'] : [] as $mood => $files) {
                foreach (array_keys($views) as $view) {
                    $path = is_array($files) ? ($files[$view] ?? null) : null;
                    if (is_string($path) && $path !== '') $views[$view][(string) $mood] = ['path' => $path, 'size' => null];
                }
            }
            ksort($views['portrait']);
            ksort($views['full']);
            $out['characters'][$id] = [
                'character' => $character,
                'persona' => $persona,
                'look' => is_string($look['id'] ?? null) ? $look['id'] : null,
                'portraits' => (object) $views['portrait'],
                'full' => (object) $views['full'],
                'form' => in_array($look['form'] ?? null, self::FORMS, true) ? $look['form'] : null,
                'model' => self::model($look['model'] ?? null) ?? self::model($personaModel),
            ];
        }
        return $out;
    }

    /** @return ?array{path: string, size: null} */
    private static function model(mixed $path): ?array
    {
        return is_string($path) && trim($path) !== '' ? ['path' => trim($path), 'size' => null] : null;
    }

    /**
     * a home scene's media field `{ src, variants? }` → a file per quality level (a level without its own variant gets
     * `src`); none, or an empty `src` → an empty map
     */
    private static function levels(mixed $media): object
    {
        $files = [];
        if (is_array($media) && is_string($media['src'] ?? null) && $media['src'] !== '') {
            $variants = is_array($media['variants'] ?? null) ? $media['variants'] : [];
            foreach (ChapterFiles::LEVELS as $level) {
                $file = $variants[$level] ?? null;
                $files[$level] = ['path' => is_string($file) && $file !== '' ? $file : $media['src'], 'size' => null];
            }
        }
        return (object) $files;
    }

    /**
     * A deck's character name → [registry id, persona id, its default look, the persona's model]: the id itself, an old VN id, or
     * `<character>-<persona>` (the first split whose left part is a character with that persona)
     *
     * @param array<string, array<mixed>> $byId
     * @return array{0: string, 1: string, 2: array<mixed>, 3: mixed}|null
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
                return [$character, $persona, $default, $p['model'] ?? null];
            }
        }
        return null;
    }

    /**
     * every file path of an answer
     * @param array<string, mixed> $answer
     * @return list<string>
     */
    public static function paths(array $answer): array
    {
        $paths = [];
        foreach ($answer['sets'] as $s) foreach (array_keys(self::SET_MEDIA) as $m) foreach ((array) ($s[$m] ?? []) as $f) $paths[] = $f['path'];
        foreach ($answer['characters'] as $c) {
            foreach ([...array_values((array) $c['portraits']), ...array_values((array) ($c['full'] ?? []))] as $f) $paths[] = $f['path'];
            if (is_array($c['model'] ?? null)) $paths[] = $c['model']['path'];
        }
        foreach ($answer['items'] ?? [] as $i) if (is_array($i['picture'] ?? null)) $paths[] = $i['picture']['path'];
        return array_values(array_unique($paths));
    }

    /**
     * @param array<string, mixed> $answer
     * @param array<string, int> $sizes path → bytes (unknown paths keep `size: null`)
     * @return array<string, mixed>
     */
    public static function withSizes(array $answer, array $sizes): array
    {
        $fill = function (object $files) use ($sizes): object {
            $out = [];
            foreach ((array) $files as $k => $f) $out[$k] = ['path' => $f['path'], 'size' => $sizes[$f['path']] ?? null];
            return (object) $out;
        };
        foreach ($answer['sets'] as $id => $s) {
            foreach (array_keys(self::SET_MEDIA) as $m) if (isset($s[$m])) $answer['sets'][$id][$m] = $fill($s[$m]);
        }
        foreach ($answer['characters'] as $id => $c) {
            $answer['characters'][$id]['portraits'] = $fill($c['portraits']);
            if (isset($c['full'])) $answer['characters'][$id]['full'] = $fill($c['full']);
            if (is_array($c['model'] ?? null)) $answer['characters'][$id]['model'] = ['path' => $c['model']['path'], 'size' => $sizes[$c['model']['path']] ?? null];
        }
        foreach ($answer['items'] ?? [] as $id => $i) {
            if (is_array($i['picture'] ?? null)) $answer['items'][$id]['picture'] = ['path' => $i['picture']['path'], 'size' => $sizes[$i['picture']['path']] ?? null];
        }
        return $answer;
    }
}
