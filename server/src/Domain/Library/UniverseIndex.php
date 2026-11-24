<?php
declare(strict_types=1);

namespace Scutoons\Domain\Library;

use Scutoons\Domain\Assets\AssetPaths;
use Scutoons\Domain\Assets\AssetRegistry;
use Scutoons\Domain\Assets\AssetUsages;
use Scutoons\Domain\Catalog\CatalogStore;
use Scutoons\Domain\Storyboard\Generators\GeneratorRegistry;
use Scutoons\Generated\ContentTables;
use Scutoons\Generated\LibraryTypes;
use Scutoons\Infrastructure\AssetStorage\AssetStorages;
use Scutoons\Infrastructure\Cache\Cache;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Time;
use stdClass;

/**
 * Everything the extended Library lists (#864, docs/API.md "The extended Library"), for every member alike: the things
 * (newest versions – the creator authors against drafts, as the address book), the registry's files, who uses what, a
 * character's and a toon's levels, the casts. Built once per fingerprint – the ids, counts and times of every table it
 * reads, so a save, publish, upload or rescan gives a new one and nothing is invalidated – and cached under it. Rights
 * and places are ExtendedLibrary's, at query time.
 *
 * @phpstan-import-type ThingType from LibraryTypes
 * @phpstan-import-type ItemSubtype from LibraryTypes
 * @phpstan-import-type FileKind from LibraryTypes
 * @phpstan-import-type FileOrigin from LibraryTypes
 * @phpstan-import-type LinkRole from LibraryTypes
 * @phpstan-import-type LevelKind from LibraryTypes
 * @phpstan-type Labels array<string, string>|string
 * @phpstan-type Thing array{type: ThingType, subtype?: ItemSubtype, id: string, labels: Labels, picture: ?string, parent?: string, toons: list<string>, updated: ?string, read: string}
 * @phpstan-type File array{kind: FileKind, name: string, picture: ?string, toons: list<string>, refs: list<string>, updated: ?string, origin: FileOrigin}
 * @phpstan-type Level array{address: string, level: LevelKind, labels: Labels, picture: ?string, parent: string, mood?: string}
 * @phpstan-type Edge array{0: string, 1: LinkRole}
 * @phpstan-type Index array{things: array<string, Thing>, files: array<string, File>, uses: array<string, list<Edge>>, usedBy: array<string, list<Edge>>, fileUses: array<string, list<string>>, levels: array<string, list<Level>>, casts: array<string, array<string, list<string>>>, docs: array<string, Labels>, sources: array<string, string>}
 */
final class UniverseIndex
{
    private const CACHE_TTL = 86400;
    /** bump when the index's shape changes: old cache entries are never read again */
    private const SHAPE = 1;
    /** who may read a thing (as reading it today): the team (content editors and the `live` role), the `live` role, content editors */
    public const TEAM = 'team';
    public const LIVE = 'live';
    public const EDITOR = 'editor';
    /** the entity kinds the index reads (newest versions) */
    private const ENTITY_KINDS = ['toon', 'character', 'place', 'scene', 'show', 'cast', 'chapter', 'season', 'credits', 'people', 'kanon'];
    /** catalog kind → the item subtype it lists as (null: a plain item) */
    private const ITEMS = ['items' => null, 'recipes' => 'recipe', 'products' => 'product', 'editions' => 'edition', 'albums' => 'album', 'home-scenes' => 'home-scene'];
    /** the other catalog kinds that are things */
    private const SOUND = ['modules' => 'module', 'instruments' => 'instrument', 'rigs' => 'rig'];
    /** a thing's address in a document's text (a scene's `ref`, a show's piece): kind, id */
    private const ADDRESS_IN_TEXT = '/"(character|location|place|item|recipe|product|edition|album|home-scene|scene|show|module|instrument|rig|toon):([A-Za-z0-9][A-Za-z0-9_.-]{0,99})"/';
    /** what a catalog entry's picture is: the first of these fields with a path */
    private const PICTURE_FIELDS = ['icon', 'image', 'cover', 'picture', 'thumbnail'];

    /** @var array<string, array{picture: ?string}> CDN path → the file it is a rendition of (built with the files) */
    private array $byPath = [];
    private string $cdn = '';

    public function __construct(
        private readonly Database $db,
        private readonly CatalogStore $catalog,
        private readonly AssetRegistry $registry,
        private readonly AssetUsages $usages,
        private readonly AssetStorages $storages,
        private readonly GeneratorRegistry $generators,
        private readonly Cache $cache,
    ) {
    }

    /** What the index is built from: changes with every save, publish, catalog write, upload and usage rescan */
    public function fingerprint(): string
    {
        $parts = ['shape' => self::SHAPE, 'format' => LibraryTypes::FORMAT];
        $parts['entities'] = $this->db->one("SELECT MAX(id) AS top, COUNT(*) AS n, COALESCE(SUM(CASE WHEN status = 'published' THEN id ELSE 0 END), 0) AS live FROM entity_versions");
        foreach ([...array_keys(self::ITEMS), ...array_keys(self::SOUND)] as $kind) {
            $parts[$kind] = $this->db->one('SELECT COUNT(*) AS n, MAX(updated_at) AS at FROM ' . CatalogStore::table($kind));
        }
        if ($this->registry->available()) {
            $parts['assets'] = $this->db->one("SELECT (SELECT COUNT(*) FROM assets) AS n, (SELECT MAX(updated_at) FROM assets) AS at, (SELECT MAX(id) FROM asset_versions) AS versions,
                (SELECT COUNT(*) FROM asset_renditions) AS files, (SELECT MAX(id) FROM asset_renditions) AS file, (SELECT COUNT(*) FROM asset_usages) AS uses, (SELECT MAX(id) FROM asset_usages) AS used");
        }
        $parts['generators'] = array_map(fn(array $g) => [$g['id'], $g['label']], $this->generators->describe());
        $parts['cdn'] = $this->storages->primaryPublicBase();
        return substr(sha1((string) json_encode($parts)), 0, 24);
    }

    /** @return Index */
    public function index(string $fingerprint): array
    {
        $key = "library-index:$fingerprint";
        $cached = $this->cache->get($key);
        if ($cached !== null) {
            /** @var Index $index the shape this class wrote under SHAPE */
            $index = json_decode($cached, true);
            if (is_array($index)) return $index;
        }
        $index = $this->build();
        $this->cache->set($key, (string) json_encode($index, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE), self::CACHE_TTL);
        return $index;
    }

    /** @return Index */
    private function build(): array
    {
        $this->cdn = rtrim($this->storages->primaryPublicBase(), '/');
        $files = $this->files();
        $docs = $this->entities();
        $things = [];
        $levels = [];
        $edges = [];
        $casts = [];
        $names = [];
        $sources = [];

        // toons first: their source language picks a name, their folder resolves their paths
        foreach ($docs as $key => ['doc' => $doc, 'at' => $at]) {
            if (!str_starts_with($key, 'toon:') || !$doc instanceof stdClass || ($doc->removed ?? false) === true) continue;
            $id = substr($key, 5);
            [$sources[$id]] = Library::languages($doc, []);
            $tile = ($doc->tile ?? null) instanceof stdClass ? $doc->tile->image ?? null : null;
            $picture = $this->firstPath([$tile, $doc->avatar ?? null, $doc->thumbnail ?? null], $id);
            $things[$key] = ['type' => 'toon', 'id' => $id, 'labels' => self::labels($doc->title ?? null), 'picture' => $picture, 'toons' => [$id], 'updated' => $at, 'read' => self::TEAM];
        }
        $isToon = fn(string $id): bool => isset($things["toon:$id"]);

        // the casts: a season's characters
        foreach ($docs as $key => ['doc' => $doc]) {
            if (!str_starts_with($key, 'cast:') || !$doc instanceof stdClass) continue;
            $toon = substr($key, 5);
            foreach (($doc->seasons ?? null) instanceof stdClass ? get_object_vars($doc->seasons) : [] as $season => $list) {
                foreach (is_array($list) ? $list : [] as $entry) {
                    if (!$entry instanceof stdClass || !is_string($entry->character ?? null)) continue;
                    $character = 'character:' . $entry->character;
                    $casts[$toon][(string) $season][] = $character;
                    $edges[] = ["season:$toon:$season", $character, 'cast'];
                }
            }
        }
        $castToons = [];
        foreach ($casts as $toon => $seasons) {
            foreach ($seasons as $list) foreach ($list as $c) $castToons[$c][$toon] = true;
        }

        foreach ($docs as $key => ['doc' => $doc, 'at' => $at]) {
            if (!$doc instanceof stdClass || ($doc->removed ?? false) === true) continue;
            [$kind, $id] = explode(':', $key, 2) + [1 => ''];
            switch ($kind) {
                case 'character':
                    $toons = array_keys($castToons[$key] ?? []);
                    if (is_string($doc->own_toon ?? null) && !in_array($doc->own_toon, $toons, true)) $toons[] = $doc->own_toon;
                    [$picture, $levels[$key]] = $this->characterLevels($key, $doc);
                    $things[$key] = ['type' => 'character', 'id' => $id, 'labels' => self::labels($doc->name ?? null), 'picture' => $picture, 'toons' => self::strings($toons), 'updated' => $at, 'read' => self::TEAM];
                    break;
                case 'place':
                    $image = $doc->image ?? null;
                    $picture = $this->firstPath([$image instanceof stdClass ? $image->src ?? null : $image], '');
                    $things[$key] = ['type' => 'place', 'id' => $id, 'labels' => self::labels($doc->name ?? null), 'picture' => $picture, 'toons' => self::strings($doc->toons ?? []), 'updated' => $at, 'read' => self::TEAM];
                    break;
                case 'scene':
                case 'show':
                    $stage = $doc->stage ?? null;
                    $name = $stage instanceof stdClass && is_string($stage->name ?? null) && trim($stage->name) !== '' && $stage->name !== 'untitled' ? $stage->name : ($doc->name ?? null);
                    $things[$key] = ['type' => $kind === 'show' ? 'show' : 'scene', 'id' => $id, 'labels' => self::labels($name), 'picture' => null, 'toons' => self::strings($doc->toons ?? []), 'updated' => $at, 'read' => $kind === 'show' ? self::LIVE : self::TEAM];
                    foreach (self::addressesIn((string) json_encode($doc)) as $used) $edges[] = [$key, $used, 'reference'];
                    break;
                case 'chapter':
                    if (preg_match('/^([^:]+):([0-9]+)$/', $id, $m) !== 1) break;
                    $names[$key] = self::labels($doc->title ?? null);
                    foreach (self::fields($doc, ['character', 'item']) as [$field, $value]) {
                        $value = $field === 'character' ? ContentTables::LEGACY_CHARACTER_IDS[$value]['character'] ?? $value : $value;
                        $edges[] = [$key, "$field:$value", 'reference'];
                    }
                    foreach (self::addressesIn((string) json_encode($doc)) as $used) $edges[] = [$key, $used, 'reference'];
                    break;
                case 'season':
                    if (preg_match('/^([^:]+):([0-9]+)$/', $id) === 1) $names[$key] = self::labels($doc->title ?? null);
                    break;
                case 'credits':
                    foreach (self::fields($doc, ['person']) as [, $person]) $edges[] = ["toon:$id", "person:$person", 'credits'];
                    break;
                case 'people':
                    foreach (is_array($doc->people ?? null) ? $doc->people : [] as $p) {
                        if (!$p instanceof stdClass || !is_string($p->id ?? null) || isset($p->merged_into)) continue;
                        $things["person:$p->id"] = ['type' => 'person', 'id' => $p->id, 'labels' => self::labels($p->name ?? null), 'picture' => null, 'toons' => [], 'updated' => $at, 'read' => self::EDITOR];
                    }
                    break;
                case 'kanon':
                    $parts = explode(':', $id);
                    $subject = explode('.', $parts[0]);
                    $parent = match ($subject[0]) { 'toon' => 'toon:' . ($subject[1] ?? ''), 'place' => 'place:' . ($subject[1] ?? ''), 'character' => 'character:' . ($subject[1] ?? ''), default => null };
                    $label = $doc->title ?? null;
                    $things[$key] = ['type' => 'kanon', 'id' => $id, 'labels' => is_string($label) || $label instanceof stdClass ? self::labels($label) : $parts[0] . ' (' . ($parts[1] ?? '') . ')', 'picture' => null]
                        + ($parent !== null ? ['parent' => $parent] : []) + ['toons' => [], 'updated' => $at, 'read' => self::EDITOR];
                    break;
            }
        }

        // the catalog: items and their subtypes, modules, instruments, rigs
        foreach (self::ITEMS as $catalogKind => $subtype) {
            $label = CatalogStore::KINDS[$catalogKind]['label'];
            foreach ($this->catalog->rows($catalogKind) as $row) {
                $data = CatalogStore::reader($row['data'], false);
                if (!$data instanceof stdClass) continue;
                $address = "$label:{$row['id']}";
                $toon = is_string($data->toon ?? null) && $data->toon !== '' ? [$data->toon] : [];
                $things[$address] = ['type' => 'item'] + ($subtype !== null ? ['subtype' => $subtype] : [])
                    + ['id' => $row['id'], 'labels' => self::labels($data->name ?? $data->title ?? null), 'picture' => $this->catalogPicture($data), 'toons' => $toon, 'updated' => Time::iso($row['updated_at']), 'read' => self::TEAM];
                if (is_string($data->item_id ?? null)) $edges[] = [$address, "item:$data->item_id", 'reference'];
                if (is_string($data->edition_id ?? null)) $edges[] = [$address, "edition:$data->edition_id", 'reference'];
                if ($catalogKind === 'recipes') foreach (self::fields($data, ['item']) as [, $item]) $edges[] = [$address, "item:$item", 'reference'];
            }
        }
        foreach (self::SOUND as $catalogKind => $type) {
            foreach ($this->catalog->rows($catalogKind) as $row) {
                $data = CatalogStore::reader($row['data'], false);
                if (!$data instanceof stdClass) continue;
                $address = "$type:{$row['id']}";
                $thing = ['type' => $type, 'id' => $row['id'], 'labels' => self::labels($data->name ?? null), 'picture' => null, 'toons' => [], 'updated' => Time::iso($row['updated_at']), 'read' => self::TEAM];
                if ($type === 'rig') {
                    // a rig's subject: `character.<id>[.<persona>]` or `item.<id>` – the thing that plays it
                    $subject = explode('.', $row['id']);
                    $owner = in_array($subject[0], ['character', 'item'], true) && isset($subject[1]) ? "$subject[0]:$subject[1]" : null;
                    if ($owner !== null) {
                        $thing['parent'] = $owner;
                        $edges[] = [$owner, $address, 'rig'];
                    }
                    foreach (is_array($data->track ?? null) ? $data->track : [] as $track) {
                        if ($track instanceof stdClass && is_string($track->instrument ?? null)) $edges[] = [$address, "instrument:$track->instrument", 'rig'];
                    }
                    $policy = $data->policy ?? null;
                    if ($policy instanceof stdClass && is_string($policy->module ?? null)) $edges[] = [$address, "module:$policy->module", 'rig'];
                    if ($thing['labels'] === []) $thing['labels'] = $row['id'];
                }
                $things[$address] = $thing;
            }
        }
        foreach ($this->generators->describe() as $g) {
            $things["generator:{$g['id']}"] = ['type' => 'generator', 'id' => (string) $g['id'], 'labels' => (string) $g['label'], 'picture' => null, 'toons' => [], 'updated' => null, 'read' => self::EDITOR];
        }

        // who uses what: only between known things and the documents that hold them; a person's toons are the credits'
        $uses = [];
        $usedBy = [];
        $seen = [];
        foreach ($edges as [$from, $to, $as]) {
            if ($from === $to || !isset($things[$to]) || isset($seen["$from>$to"])) continue;
            $seen["$from>$to"] = true;
            $uses[$from][] = [$to, $as];
            $usedBy[$to][] = [$from, $as];
            if ($as === 'credits' && $things[$to]['type'] === 'person') $things[$to]['toons'] = self::strings([...$things[$to]['toons'], substr($from, 5)]);
        }
        // a rig and a Kanon text belong to their subject's toons
        foreach ($things as $address => $thing) {
            $parent = $thing['parent'] ?? null;
            if ($parent !== null && isset($things[$parent]) && $thing['toons'] === []) $things[$address]['toons'] = $things[$parent]['toons'];
        }

        // a file belongs to the toons of what uses it, and to its folder's toon
        $fileUses = [];
        foreach ($files as $address => $file) {
            $toons = $file['toons'];
            foreach ($file['refs'] as $ref) {
                $fileUses[$ref][] = $address;
                foreach (isset($things[$ref]) ? $things[$ref]['toons'] : self::toonOfKey($ref) as $t) $toons[] = $t;
            }
            $files[$address]['toons'] = self::strings(array_filter($toons, $isToon));
        }

        return ['things' => $things, 'files' => $files, 'uses' => $uses, 'usedBy' => $usedBy, 'fileUses' => $fileUses, 'levels' => array_filter($levels + $this->toonLevels($docs, $things)),
            'casts' => $casts, 'docs' => $names, 'sources' => $sources];
    }

    /**
     * The newest version of every document of the kinds the index reads, decoded, with when it was saved
     * @return array<string, array{doc: mixed, at: ?string}>
     */
    private function entities(): array
    {
        $marks = implode(',', array_fill(0, count(self::ENTITY_KINDS), '?'));
        $newest = $this->db->all("SELECT MAX(id) AS id FROM entity_versions WHERE kind IN ($marks) AND (locale IS NULL OR kind = 'kanon') GROUP BY entity_key", self::ENTITY_KINDS);
        $out = [];
        foreach (array_chunk(array_map(fn(array $r) => (int) $r['id'], $newest), 300) as $chunk) {
            foreach ($this->db->all('SELECT entity_key, json, created_at FROM entity_versions WHERE id IN (' . implode(',', array_fill(0, count($chunk), '?')) . ')', $chunk) as $r) {
                $out[(string) $r['entity_key']] = ['doc' => json_decode((string) $r['json']), 'at' => Time::iso($r['created_at'])];
            }
        }
        ksort($out);
        return $out;
    }

    /**
     * The registry's files (current versions, map tiles left out: never picked one by one), each with its smallest picture
     * and the documents that use it (library.vectors.json holds the cases)
     * @return array<string, File>
     */
    private function files(): array
    {
        if (!$this->registry->available()) return [];
        $renditions = [];
        foreach ($this->db->all("SELECT r.version_id, r.path, r.mime, r.width, r.origin FROM asset_renditions r JOIN asset_versions v ON v.id = r.version_id AND v.status = 'current'") as $r) {
            $renditions[(int) $r['version_id']][] = $r;
        }
        $refs = [];
        foreach ($this->db->all('SELECT DISTINCT path, ref FROM asset_usages') as $u) $refs[(string) $u['path']][(string) $u['ref']] = true;
        $out = [];
        $rows = $this->db->all("SELECT a.slug, a.kind, a.folder, a.title, a.updated_at, v.id AS version_id, v.path FROM assets a
            JOIN asset_versions v ON v.asset_id = a.id AND v.status = 'current' WHERE a.status = 'active' AND a.slug NOT REGEXP ?", [AssetPaths::TILE_SQL]);
        foreach ($rows as $row) {
            $list = $renditions[(int) $row['version_id']] ?? [];
            $file = self::file($row, $list, $this->cdn);
            $address = $file['kind'] . ':' . $row['path'];
            $used = [];
            foreach ($list as $r) $used += $refs[(string) $r['path']] ?? [];
            $used += $refs[(string) $row['path']] ?? [];
            $file['refs'] = array_map('strval', array_keys($used));
            $file['toons'] = is_string($row['folder']) ? [$row['folder']] : [];
            $out[$address] = $file;
            foreach ([(string) $row['path'], ...array_map(fn(array $r) => (string) $r['path'], $list)] as $path) $this->byPath[$path] = ['picture' => $file['picture']];
        }
        ksort($out);
        return $out;
    }

    /**
     * One registry asset as a Library file: its kind (archive, document, font, other are a `file`), its name (title, else
     * slug), its picture (an image's or a video's smallest image rendition, an image's own file without one; null else),
     * its origin (its main file's). The rule every reader holds to: server/schema/api/library.vectors.json.
     * @param array<string, mixed> $asset slug, kind, title, updated_at, path
     * @param list<array<string, mixed>> $renditions path, mime, width, origin
     * @return File
     */
    public static function file(array $asset, array $renditions, string $cdn): array
    {
        $kind = match ($asset['kind'] ?? null) { 'image' => 'image', 'audio' => 'audio', 'video' => 'video', 'model' => 'model', 'animation' => 'animation', default => 'file' };
        $smallest = null;
        foreach ($renditions as $r) {
            if (!str_starts_with((string) ($r['mime'] ?? ''), 'image/')) continue;
            $width = is_numeric($r['width'] ?? null) ? (int) $r['width'] : PHP_INT_MAX;
            if ($smallest === null || $width < $smallest[0]) $smallest = [$width, (string) $r['path']];
        }
        $path = (string) $asset['path'];
        $picture = match (true) {
            in_array($kind, ['image', 'video'], true) && $smallest !== null => $smallest[1],
            $kind === 'image' => $path,
            default => null,
        };
        $main = array_values(array_filter($renditions, fn(array $r) => (string) $r['path'] === $path))[0] ?? $renditions[0] ?? [];
        $origin = match ($main['origin'] ?? null) { 'uploaded' => 'uploaded', 'generated' => 'generated', default => 'imported' };
        $title = trim((string) ($asset['title'] ?? ''));
        return [
            'kind' => $kind,
            'name' => $title !== '' ? $title : (string) $asset['slug'],
            'picture' => $picture !== null ? self::url($cdn, $picture) : null,
            'toons' => [],
            'refs' => [],
            'updated' => Time::iso($asset['updated_at'] ?? null),
            'origin' => $origin,
        ];
    }

    private static function url(string $cdn, string $path): string
    {
        return $cdn === '' ? $path : rtrim($cdn, '/') . '/' . ltrim($path, '/');
    }

    /** a path a document names → the picture of the file it is (its smallest one), or the path's own URL; null without one */
    private function picture(?string $path): ?string
    {
        if ($path === null || $path === '') return null;
        return array_key_exists($path, $this->byPath) ? $this->byPath[$path]['picture'] ?? self::url($this->cdn, $path) : self::url($this->cdn, $path);
    }

    /** @param list<mixed> $candidates the first string resolved against `$base` (a toon's folder, '' = the CDN root) */
    private function firstPath(array $candidates, string $base): ?string
    {
        foreach ($candidates as $raw) {
            if (!is_string($raw) || trim($raw) === '') continue;
            [$path] = $this->usages->resolve($raw, $base);
            if ($path !== null) return $this->picture($path);
        }
        return null;
    }

    private function catalogPicture(stdClass $data): ?string
    {
        foreach (self::PICTURE_FIELDS as $field) {
            $v = $data->$field ?? null;
            $raw = $v instanceof stdClass ? $v->src ?? null : $v;
            if (is_string($raw) && trim($raw) !== '') return $this->firstPath([$raw], '');
        }
        return null;
    }

    /**
     * A character's levels – personas › looks › moods, parents first – and its own picture (the first persona's default
     * look's: its neutral mood, else the first; avatar, else portrait, else full)
     * @return array{0: ?string, 1: list<Level>}
     */
    private function characterLevels(string $character, stdClass $doc): array
    {
        $levels = [];
        $own = null;
        foreach (self::objects($doc->personas ?? null) as $persona) {
            if (!is_string($persona->id ?? null) || $persona->id === '') continue;
            $p = "$character.$persona->id";
            $below = [];
            $personaPicture = null;
            foreach (self::objects($persona->looks ?? null) as $look) {
                if (!is_string($look->id ?? null) || $look->id === '') continue;
                $l = "$p.$look->id";
                $moods = [];
                $pictures = [];
                foreach (($look->images ?? null) instanceof stdClass ? get_object_vars($look->images) : [] as $mood => $files) {
                    $picture = $files instanceof stdClass ? $this->firstPath([$files->avatar ?? null, $files->portrait ?? null, $files->full ?? null], '') : null;
                    $pictures[(string) $mood] = $picture;
                    $moods[] = ['address' => $l, 'level' => 'mood', 'labels' => (string) $mood, 'picture' => $picture, 'parent' => $l, 'mood' => (string) $mood];
                }
                $lookPicture = array_key_exists('neutral', $pictures) ? $pictures['neutral'] : (array_values($pictures)[0] ?? null);
                if ($personaPicture === null || ($look->default ?? false) === true) $personaPicture = $lookPicture ?? $personaPicture;
                array_push($below, ['address' => $l, 'level' => 'look', 'labels' => self::labels($look->label ?? null) ?: $look->id, 'picture' => $lookPicture, 'parent' => $p], ...$moods);
            }
            array_push($levels, ['address' => $p, 'level' => 'persona', 'labels' => self::labels($persona->label ?? null) ?: $persona->id, 'picture' => $personaPicture, 'parent' => $character], ...$below);
            $own ??= $personaPicture;
        }
        return [$own, $levels];
    }

    /**
     * A toon's levels: its seasons, then each season's chapters (by its sequence; chapters no season lists come last,
     * under the toon)
     * @param array<string, array{doc: mixed, at: ?string}> $docs
     * @param array<string, Thing> $things
     * @return array<string, list<Level>>
     */
    private function toonLevels(array $docs, array $things): array
    {
        $out = [];
        foreach ($things as $address => $thing) {
            if ($thing['type'] !== 'toon') continue;
            $toon = $thing['id'];
            $seasons = [];
            $chapters = [];
            foreach ($docs as $key => ['doc' => $doc]) {
                if (!$doc instanceof stdClass) continue;
                if (preg_match('/^season:' . preg_quote($toon, '/') . ':([0-9]+)$/', $key, $m) === 1) $seasons[(int) $m[1]] = $doc;
                elseif (preg_match('/^chapter:' . preg_quote($toon, '/') . ':([0-9]+)$/', $key, $m) === 1) $chapters[(int) $m[1]] = $doc;
            }
            ksort($seasons);
            ksort($chapters);
            $levels = [];
            $placed = [];
            foreach ($seasons as $n => $season) {
                $s = "season:$toon:$n";
                $levels[] = ['address' => $s, 'level' => 'season', 'labels' => self::labels($season->title ?? null) ?: (string) $n, 'picture' => null, 'parent' => $address];
                foreach (is_array($season->sequence ?? null) ? $season->sequence : [] as $item) {
                    $c = $item instanceof stdClass ? $item->number ?? null : null;
                    if (!is_int($c) && !(is_string($c) && ctype_digit($c))) continue;
                    $c = (int) $c;
                    if (!isset($chapters[$c]) || isset($placed[$c])) continue;
                    $placed[$c] = true;
                    $levels[] = ['address' => "chapter:$toon:$c", 'level' => 'chapter', 'labels' => self::labels($chapters[$c]->title ?? null) ?: (string) $c, 'picture' => null, 'parent' => $s];
                }
            }
            foreach ($chapters as $c => $chapter) {
                if (isset($placed[$c])) continue;
                $levels[] = ['address' => "chapter:$toon:$c", 'level' => 'chapter', 'labels' => self::labels($chapter->title ?? null) ?: (string) $c, 'picture' => null, 'parent' => $address];
            }
            $out[$address] = $levels;
        }
        return $out;
    }

    /**
     * The things a document's text addresses: `character:<id>[.<persona>[.<look>]]` as the character, `location:<id>` as the place
     * @return list<string>
     */
    private static function addressesIn(string $json): array
    {
        preg_match_all(self::ADDRESS_IN_TEXT, $json, $m, PREG_SET_ORDER);
        $out = [];
        foreach ($m as [, $kind, $id]) {
            if ($kind === 'location') $kind = 'place';
            if (in_array($kind, ['character', 'place', 'item', 'scene', 'show', 'toon'], true)) $id = explode('.', $id)[0];
            $out["$kind:$id"] = true;
        }
        return array_map('strval', array_keys($out));
    }

    /**
     * Every string value under one of these keys, anywhere in the document
     * @param list<string> $keys
     * @return list<array{0: string, 1: string}> [key, value]
     */
    private static function fields(mixed $node, array $keys): array
    {
        $out = [];
        $walk = function (mixed $node) use (&$walk, &$out, $keys): void {
            if ($node instanceof stdClass) $node = get_object_vars($node);
            if (!is_array($node)) return;
            foreach ($node as $k => $v) {
                if (is_string($v) && in_array($k, $keys, true) && $v !== '') $out[] = [(string) $k, $v];
                else $walk($v);
            }
        };
        $walk($node);
        return $out;
    }

    /** @return list<string> the toon a toon-scoped document key names (`chapter:catpope:3` → catpope) */
    private static function toonOfKey(string $key): array
    {
        return preg_match('/^(?:toon|chapter|season|cast|credits|storyboard|season-script|chapter-script)(?::|$)([^:]*)/', $key, $m) === 1 && $m[1] !== '' ? [$m[1]] : [];
    }

    /** @return Labels a locale map's texts, or a plain text; [] when there is none */
    private static function labels(mixed $value): array|string
    {
        if (is_string($value)) return trim($value);
        $out = [];
        if (!$value instanceof stdClass) return $out;
        foreach (get_object_vars($value) as $locale => $text) {
            if (is_string($text)) $out[(string) $locale] = $text;
        }
        return $out;
    }

    /** @return list<stdClass> */
    private static function objects(mixed $list): array
    {
        return is_array($list) ? array_values(array_filter($list, fn(mixed $v) => $v instanceof stdClass)) : [];
    }

    /** @return list<string> */
    private static function strings(mixed $list): array
    {
        return is_array($list) ? array_values(array_unique(array_filter($list, fn(mixed $v) => is_string($v) && $v !== ''))) : [];
    }

    /** a name in the asked locale (the one locale order), else `$fallback` */
    public static function name(mixed $labels, string $locale, ?string $source, string $fallback): string
    {
        if (is_string($labels)) return $labels !== '' ? $labels : $fallback;
        return ContentTables::pickText($labels, $locale, $source) ?? $fallback;
    }
}
