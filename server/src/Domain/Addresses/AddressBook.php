<?php
declare(strict_types=1);

namespace Scutoons\Domain\Addresses;

use Scutoons\Domain\Catalog\CatalogStore;
use Scutoons\Domain\Content\FlowIndex;
use Scutoons\Domain\Content\Variables;
use Scutoons\Domain\Library\Library;
use Scutoons\Generated\AddressBookTypes;
use Scutoons\Infrastructure\Cache\Cache;
use Scutoons\Infrastructure\Database\Database;
use stdClass;

/**
 * One toon's address book (#477; the shape: server/schema/api/address-book.schema.json, its PHP types AddressBookTypes):
 * every thing of the toon's catalog a typeahead may pick, read by the creator's VN events and the deck editor in one
 * typeahead each. Every entry has its **reference** (`ref`: which thing); only the kinds a receiver exists for also have an
 * **address** (who hears a word, the same text: ADDRESSED; decisions/deck.md [address]).
 *
 * - **characters** in the toon's cast (every season) or with `own_toon` = the toon: the character, each persona, each look
 *   with the moods it has art for – `character:<id>`, `character:<id>.<persona>`, `character:<id>.<persona>.<look>`
 *   (address.rs `Target`, SHOW.md §8); addresses
 * - **locations**: the world map's places whose `toons` hold the toon – `location:<place>` (SHOW.md §8); references
 * - **regions**: every region of the world map (one map for all toons) – `region:<id>`; references
 * - **items**: the catalog's items of the toon and the shared ones (no toon) – `item:<id>`; addresses
 * - **seasons**: the toon's seasons – `season:<number>`; references
 * - **chapters**: the toon's chapters – `chapter:<number>`, each followed by its **groups** – `chapter:<number>.<id>`; references
 * - **products**: the shop's products of the toon and the shared ones – `product:<id>`; references
 * - **toons**: every toon, this one first – `toon:<id>`; references (a jump's other toon; XR targets have no catalog to list)
 * - **scenes** (#487, research/SCENES.md §3 / §8): the deck scenes whose published version names the toon in its `toons` –
 *   `scene:<id>`, by id; addresses (a deck plays one: a show cell's piece). Only the published version counts – a draft never
 *   leaves the editor – and it gives the label (its stage's name, under the source language), the set and spot of its
 *   `space` node where it says them, its stamped `version` and its `placeholders` (#504: id, fill, label, preview – what a
 *   use `{ scene, with }` may fill). A template – a scene with no toon (the core's `is_template`, #509: `scene:place`, `scene:solo`,
 *   `scene:stage`, `scene:avatar-home`, SCENES.md §9.5) – is in every toon's book; the core and
 *   this book agree by the shared vectors book-entries.json.
 *   `scene` is the deck scene only: no chapter group, screenplay heading or sandbox is ever listed as one (sandboxes are
 *   files, never in entity_versions)
 * - **variables**: every story variable of the one save game (the flow index's, every toon may set any) –
 *   `variable:<scope>.<name>`; references
 * A group is its chapter's `chapter:<number>.<id>`: its `name` is the group's `id` (slug-safe, unique in the chapter, the
 * value a jump stores). The variable form is a proposal the owner has not confirmed (#477, slice B). A variable's `name` is
 * the variable as written in this toon (`coins`, `global.coins`, `toptoy.coins`).
 * Not in it yet: sets (no toon in the catalog), XR targets (no catalog). Removed characters, places,
 * regions and toons (tombstones) are left out.
 *
 * Each document's newest version counts (drafts included: the creator and the editor author against them); `origin.state`
 * says whether that version is the published one (scenes excepted: only their published version is listed). Built on request and cached under the version ids it was built from
 * (content-addressed, like FlowIndex: nothing to invalidate) – that hash is the document's `version` and the ETag.
 *
 * @phpstan-import-type AddressBook from AddressBookTypes as Book
 * @phpstan-import-type AddressEntry from AddressBookTypes
 * @phpstan-import-type LocaleText from AddressBookTypes
 * @phpstan-import-type OriginState from AddressBookTypes
 * @phpstan-import-type ScenePlaceholder from AddressBookTypes
 */
final class AddressBook
{
    private const CACHE_TTL = 604800;
    /**
     * The kinds whose entries have an address: a receiver of that type exists (owner 2026-11-10 [address]: "sending something
     * somewhere requires a possible receiver of the same type") – a character and an item (deck-space far.rs `offers`), a
     * persona and a look (the show's addressing, `picks_up`), a scene (a deck plays it, #487). Every other kind is a reference only.
     */
    private const ADDRESSED = ['character', 'persona', 'look', 'item', 'scene'];
    /** a chapter group's id (@scutoons/content GROUP_ID_PATTERN, the chapter schema's pattern) */
    private const GROUP_ID = '/^[A-Za-z0-9][A-Za-z0-9_-]*$/';
    private const JSON_FLAGS = JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_THROW_ON_ERROR;

    public function __construct(private readonly Database $db, private readonly CatalogStore $catalog, private readonly Cache $cache, private readonly FlowIndex $flow)
    {
    }

    /**
     * The document on the wire (labels as JSON objects) and its version; null when the toon has no `toon:<toon>` document
     * @return array{version: string, json: string}|null
     */
    public function document(string $toon): ?array
    {
        $scan = $this->scan([$toon]);
        if ($scan === null) return null;
        $part = $scan['parts'][$toon];
        $key = "address-book:$toon:{$part['version']}";
        $cached = $this->cache->get($key);
        if ($cached !== null) return ['version' => $part['version'], 'json' => $cached];
        $book = $this->compose($toon, $part['version'], $this->documents($scan['versions']), $part['items'], $part['products'], $scan['variables'], $part['scenes']);
        $json = json_encode(self::wire($book), self::JSON_FLAGS);
        $this->cache->set($key, $json, self::CACHE_TTL);
        return ['version' => $part['version'], 'json' => $json];
    }

    /**
     * Several toons' books in one answer (#708; the shape: AddressBooks of the schema): one scan of the newest versions,
     * each document decoded once; what every one of these books holds alike (regions, toons, templates, variables, the shared
     * items and products) once in `shared`, as the home toon's book writes it, then a section per toon with the rest of its
     * book, in the asked order (the first is the home). One version over every toon's.
     * @param list<string> $toons
     * @return array{version: string, json: string}|string the document, or the first toon that has no `toon:<toon>` document
     */
    public function books(array $toons): array|string
    {
        $scan = $this->scan($toons);
        if ($scan === null) {
            foreach ($toons as $t) if ($this->scan([$t]) === null) return $t;
            return $toons[0];
        }
        $version = substr(sha1(AddressBookTypes::FORMAT . 'books' . json_encode(array_map(fn(array $p) => $p['version'], $scan['parts']))), 0, 20);
        $key = "address-books:$version";
        $cached = $this->cache->get($key);
        if ($cached !== null) return ['version' => $version, 'json' => $cached];
        $docs = $this->documents($scan['versions']);
        $shared = [];
        foreach ($scan['parts'] as $part) {
            foreach ($part['items'] as $r) if (self::catalogToon($r['data']) === null) $shared["item:{$r['id']}"] = true;
            foreach ($part['products'] as $r) if (self::catalogToon($r['data']) === null) $shared["product:{$r['id']}"] = true;
            foreach ($part['scenes'] as $id => $s) if (!is_array($s['doc']->toons ?? null) || $s['doc']->toons === []) $shared["scene:$id"] = true;
        }
        $isShared = fn(array $e): bool => in_array($e['kind'], ['region', 'toon', 'variable'], true) || isset($shared[$e['ref']]);
        $out = ['address_book' => AddressBookTypes::FORMAT, 'toons' => $toons, 'version' => $version, 'shared' => [], 'books' => []];
        foreach ($toons as $i => $toon) {
            $part = $scan['parts'][$toon];
            $book = self::wire($this->compose($toon, $part['version'], $docs, $part['items'], $part['products'], $scan['variables'], $part['scenes']));
            if ($i === 0) $out['shared'] = array_values(array_filter($book['entries'], $isShared));
            $out['books'][] = ['toon' => $toon, 'source' => $book['source'], 'entries' => array_values(array_filter($book['entries'], fn(array $e) => !$isShared($e)))];
        }
        $json = json_encode($out, self::JSON_FLAGS);
        $this->cache->set($key, $json, self::CACHE_TTL);
        return ['version' => $version, 'json' => $json];
    }

    /**
     * The references of a toon's book (every entry's `ref`) – what the core checks a use of a scene against (#504,
     * SceneUses); null when the toon has no `toon:<toon>` document
     * @return list<string>|null
     */
    public function refs(string $toon): ?array
    {
        $doc = $this->document($toon);
        if ($doc === null) return null;
        /** @var array{entries: list<array{ref: string}>} $book */
        $book = json_decode($doc['json'], true);
        return array_column($book['entries'], 'ref');
    }

    /**
     * What the books of `$toons` are built from, read once: the newest and published version ids of every document any of
     * them reads (each toon's cast, chapters and seasons; every toon, character, place and region), the catalog, the
     * variables and the published scenes – and per toon its share of it and its version (the hash of the ids it was built
     * from: content-addressed, nothing to invalidate). Null when a toon has no `toon:<toon>` document.
     * @param list<string> $toons
     * @return array{versions: array<string, array{0: int, 1: ?int}>, variables: array<string, array{set: list<string>, read: list<string>}>, parts: array<string, array{version: string, items: list<array{id: string, position: int, data: string, updated_at: ?string, updated_by: ?string}>, products: list<array{id: string, position: int, data: string, updated_at: ?string, updated_by: ?string}>, scenes: array<string, array{row: int, doc: stdClass}>}>}|null
     */
    private function scan(array $toons): ?array
    {
        $rows = $this->db->all(
            "SELECT entity_key, MAX(id) AS latest, MAX(CASE WHEN status = 'published' THEN id END) AS published FROM entity_versions
             WHERE locale IS NULL AND ((kind IN ('cast', 'chapter', 'season') AND toon IN (" . implode(',', array_fill(0, count($toons), '?')) . ")) OR kind IN ('toon', 'character', 'place', 'region'))
             GROUP BY entity_key ORDER BY entity_key",
            $toons
        );
        $versions = [];
        foreach ($rows as $r) $versions[(string) $r['entity_key']] = [(int) $r['latest'], $r['published'] !== null ? (int) $r['published'] : null];
        foreach ($toons as $toon) if (!isset($versions["toon:$toon"])) return null;
        $allItems = $this->catalog->rows('items');
        $allProducts = $this->catalog->rows('products');
        $variables = $this->variables();
        $allScenes = $this->scenes();
        $hashes = fn(array $list) => array_map(fn(array $r) => [$r['id'], sha1($r['data'])], $list);
        $parts = [];
        foreach ($toons as $toon) {
            $mine = fn(array $r) => in_array(self::catalogToon($r['data']), [$toon, null], true);
            $items = array_values(array_filter($allItems, $mine));
            $products = array_values(array_filter($allProducts, $mine));
            $scenes = array_filter($allScenes, fn(array $s) => !is_array($s['doc']->toons ?? null) || $s['doc']->toons === [] || in_array($toon, $s['doc']->toons, true));
            $own = array_filter($versions, fn(string $key) => self::readBy($key, $toon), ARRAY_FILTER_USE_KEY);
            $version = substr(sha1(AddressBookTypes::FORMAT . json_encode([$own, $hashes($items), $hashes($products), $variables, array_map(fn(array $s) => $s['row'], $scenes)])), 0, 20);
            $parts[$toon] = ['version' => $version, 'items' => $items, 'products' => $products, 'scenes' => $scenes];
        }
        return ['versions' => $versions, 'variables' => $variables, 'parts' => $parts];
    }

    /** whether `$toon`'s book reads the document `$key`: its own cast, chapters and seasons, and every toon, character, place and region */
    private static function readBy(string $key, string $toon): bool
    {
        return $key === "cast:$toon" || str_starts_with($key, "chapter:$toon:") || str_starts_with($key, "season:$toon:")
            || preg_match('#^(toon|character|place|region):#', $key) === 1;
    }

    /**
     * @param array<string, array{0: int, 1: ?int}> $versions entity key → [newest id, published id]
     * @return array<string, array{doc: mixed, state: OriginState}> entity key → its newest document and whether it is published
     */
    private function documents(array $versions): array
    {
        $ids = array_column($versions, 0);
        $json = [];
        foreach (array_chunk($ids, 500) as $chunk) {
            foreach ($this->db->all('SELECT id, json FROM entity_versions WHERE id IN (' . implode(',', array_fill(0, count($chunk), '?')) . ')', $chunk) as $r) {
                $json[(int) $r['id']] = (string) $r['json'];
            }
        }
        $out = [];
        foreach ($versions as $key => [$latest, $published]) {
            $out[$key] = ['doc' => json_decode($json[$latest] ?? 'null'), 'state' => $latest === $published ? 'published' : 'draft'];
        }
        return $out;
    }

    /**
     * Every published deck scene by id (a draft never leaves the editor); a toon's book takes the ones whose `toons` name
     * it, and the templates – published scenes with no toon (#504, SCENES.md §9.5, owner 2026-11-12 [scenes]: "in every
     * toon's book"; the core's one definition, scene_document.rs `is_template`, held to this by the shared vectors
     * book-entries.json, #509)
     * @return array<string, array{row: int, doc: stdClass}> scene id → the published version's row id and document
     */
    private function scenes(): array
    {
        $out = [];
        foreach ($this->db->all("SELECT id, entity_key, json FROM entity_versions WHERE kind = 'scene' AND status = 'published' AND locale IS NULL ORDER BY entity_key") as $r) {
            $doc = json_decode((string) $r['json']);
            if (!$doc instanceof stdClass) continue;
            $out[substr((string) $r['entity_key'], strlen('scene:'))] = ['row' => (int) $r['id'], 'doc' => $doc];
        }
        return $out;
    }

    /**
     * The story variables of the one save game (FlowIndex, built from every toon's newest documents and cached there):
     * storage key → the toons that set it, the toons that read it
     * @return array<string, array{set: list<string>, read: list<string>}>
     */
    private function variables(): array
    {
        $out = [];
        foreach ((array) ($this->flow->build()['variables'] ?? []) as $key => $v) {
            $v = (array) $v;
            $out[(string) $key] = ['set' => array_values(array_map('strval', (array) ($v['set'] ?? []))), 'read' => array_values(array_map('strval', (array) ($v['read'] ?? [])))];
        }
        return $out;
    }

    /**
     * @param array<string, array{doc: mixed, state: OriginState}> $docs
     * @param list<array{id: string, position: int, data: string, updated_at: ?string, updated_by: ?string}> $items
     * @param list<array{id: string, position: int, data: string, updated_at: ?string, updated_by: ?string}> $products
     * @param array<string, array{set: list<string>, read: list<string>}> $variables
     * @param array<string, array{row: int, doc: stdClass}> $scenes
     * @return Book
     */
    private function compose(string $toon, string $version, array $docs, array $items, array $products, array $variables, array $scenes): array
    {
        $toonDoc = $docs["toon:$toon"]['doc'];
        [$source] = Library::languages($toonDoc instanceof stdClass ? $toonDoc : null, []);
        $entries = [];
        foreach ($this->characters($toon, $docs) as $id) {
            ['doc' => $doc, 'state' => $state] = $docs["character:$id"];
            $origin = ['key' => "character:$id", 'state' => $state];
            $entries[] = ['ref' => "character:$id", 'kind' => 'character', 'name' => $id, 'labels' => self::text($doc->name ?? null), 'origin' => $origin];
            foreach (self::objects($doc->personas ?? null) as $persona) {
                if (!is_string($persona->id ?? null) || $persona->id === '') continue;
                $p = "character:$id.$persona->id";
                $entries[] = ['ref' => $p, 'kind' => 'persona', 'name' => $persona->id, 'parent' => "character:$id", 'labels' => self::text($persona->label ?? null), 'origin' => $origin];
                foreach (self::objects($persona->looks ?? null) as $look) {
                    if (!is_string($look->id ?? null) || $look->id === '') continue;
                    $moods = ($look->images ?? null) instanceof stdClass ? array_map('strval', array_keys(get_object_vars($look->images))) : [];
                    $entries[] = ['ref' => "$p.$look->id", 'kind' => 'look', 'name' => $look->id, 'parent' => $p, 'labels' => self::text($look->label ?? null), 'origin' => $origin, 'moods' => $moods];
                }
            }
        }
        foreach ($docs as $key => ['doc' => $doc, 'state' => $state]) {
            if (!str_starts_with($key, 'place:') || !$doc instanceof stdClass || ($doc->removed ?? false) === true) continue;
            if (!in_array($toon, is_array($doc->toons ?? null) ? $doc->toons : [], true)) continue;
            $id = substr($key, strlen('place:'));
            $entries[] = ['ref' => "location:$id", 'kind' => 'location', 'name' => $id, 'labels' => self::text($doc->name ?? null), 'origin' => ['key' => $key, 'state' => $state]];
        }
        // the world map's regions (one map for every toon; where migrate registers someone)
        foreach ($docs as $key => ['doc' => $doc, 'state' => $state]) {
            if (!str_starts_with($key, 'region:') || !$doc instanceof stdClass || ($doc->removed ?? false) === true) continue;
            $id = substr($key, strlen('region:'));
            $entries[] = ['ref' => $key, 'kind' => 'region', 'name' => $id, 'labels' => self::text($doc->name ?? null), 'origin' => ['key' => $key, 'state' => $state]];
        }
        foreach ($items as $row) {
            $item = CatalogStore::reader($row['data'], false);
            $entries[] = ['ref' => "item:{$row['id']}", 'kind' => 'item', 'name' => $row['id'], 'labels' => self::text($item instanceof stdClass ? $item->name ?? null : null), 'origin' => ['key' => "item:{$row['id']}", 'state' => 'published']];
        }
        array_push($entries, ...self::seasons($docs, $toon));
        $chapters = [];
        foreach ($docs as $key => ['doc' => $doc, 'state' => $state]) {
            if (preg_match('#^chapter:' . preg_quote($toon, '#') . ':([0-9]+)$#', $key, $m) !== 1) continue;
            $origin = ['key' => $key, 'state' => $state];
            $chapter = "chapter:$m[1]";
            $chapters[(int) $m[1]] = [['ref' => $chapter, 'kind' => 'chapter', 'name' => $m[1], 'labels' => self::text($doc->title ?? null), 'origin' => $origin]];
            $ids = [];
            foreach (self::objects($doc instanceof stdClass ? $doc->groups ?? null : null) as $group) {
                $id = $group->id ?? null;
                if (!is_string($id) || preg_match(self::GROUP_ID, $id) !== 1 || isset($ids[$id])) continue;
                $ids[$id] = true;
                $chapters[(int) $m[1]][] = ['ref' => "$chapter.$id", 'kind' => 'group', 'name' => $id, 'parent' => $chapter, 'labels' => [], 'origin' => $origin];
            }
        }
        ksort($chapters);
        foreach ($chapters as $list) array_push($entries, ...$list);
        // products (a jump's target `/shop/product/<id>`): the toon's and the shared ones, like items. A product's name is one
        // text, no locale map: it stands under the toon's source language.
        foreach ($products as $row) {
            $product = CatalogStore::reader($row['data'], false);
            $name = $product instanceof stdClass ? $product->name ?? null : null;
            $labels = is_string($name) && $name !== '' ? [$source => $name] : self::text($name);
            $entries[] = ['ref' => "product:{$row['id']}", 'kind' => 'product', 'name' => $row['id'], 'labels' => $labels, 'origin' => ['key' => "product:{$row['id']}", 'state' => 'published']];
        }
        array_push($entries, ...self::toons($docs, $toon));
        foreach ($scenes as $id => ['doc' => $doc]) $entries[] = self::scene((string) $id, $doc, $source);
        // variables (proposal, #477): the scope is the owning toon or `global`; names that can't be written in a script
        // (the reader's own `picked:…` keys) are no targets. The flow index keeps no draft state per variable: `published`.
        foreach ($variables as $key => $v) {
            $written = Variables::written($key, $toon);
            if (preg_match(Variables::WRITTEN, $written) !== 1) continue;
            $from = $v['set'][0] ?? $v['read'][0] ?? $toon;
            $entries[] = ['ref' => 'variable:' . Variables::scope($key) . '.' . Variables::name($key), 'kind' => 'variable', 'name' => $written, 'labels' => [], 'origin' => ['key' => "toon:$from", 'state' => 'published']];
        }
        // the address beside the reference, where a receiver of the kind exists
        $entries = array_map(fn(array $e): array => in_array($e['kind'], self::ADDRESSED, true) ? ['ref' => $e['ref'], 'address' => $e['ref']] + $e : $e, $entries);
        return ['address_book' => AddressBookTypes::FORMAT, 'toon' => $toon, 'version' => $version, 'source' => $source, 'entries' => $entries];
    }

    /**
     * The toon's seasons by number (a jump's target `/<toon>/season/<n>`) – `season:<number>`, labelled by the title
     * @param array<string, array{doc: mixed, state: OriginState}> $docs
     * @return list<AddressEntry>
     */
    private static function seasons(array $docs, string $toon): array
    {
        $out = [];
        foreach ($docs as $key => ['doc' => $doc, 'state' => $state]) {
            if (preg_match('#^season:' . preg_quote($toon, '#') . ':([0-9]+)$#', $key, $m) !== 1 || !$doc instanceof stdClass) continue;
            $out[(int) $m[1]] = ['ref' => "season:$m[1]", 'kind' => 'season', 'name' => $m[1], 'labels' => self::text($doc->title ?? null), 'origin' => ['key' => $key, 'state' => $state]];
        }
        ksort($out);
        return array_values($out);
    }

    /**
     * Every toon – `toon:<id>`, labelled by its title, this toon first: a jump's target `toon` (its page), and in the creator's
     * jump field the way into another toon's own book (#477 slice D)
     * @param array<string, array{doc: mixed, state: OriginState}> $docs
     * @return list<AddressEntry>
     */
    private static function toons(array $docs, string $toon): array
    {
        $out = [];
        foreach ($docs as $key => ['doc' => $doc, 'state' => $state]) {
            if (!str_starts_with($key, 'toon:') || !$doc instanceof stdClass || ($doc->removed ?? false) === true) continue;
            $id = substr($key, strlen('toon:'));
            $out[$id === $toon ? '' : $key] = ['ref' => $key, 'kind' => 'toon', 'name' => $id, 'labels' => self::text($doc->title ?? null), 'origin' => ['key' => $key, 'state' => $state]];
        }
        ksort($out, SORT_STRING);
        return array_values($out);
    }

    /**
     * A published scene's entry: labelled by its stage's name (one text, no locale map: under the source language; the
     * schema's default `untitled` is no name), the set and spot its `space` node says, its stamped version
     * @return AddressEntry
     */
    private static function scene(string $id, stdClass $doc, string $source): array
    {
        $name = ($doc->stage ?? null) instanceof stdClass ? $doc->stage->name ?? null : null;
        $labels = is_string($name) && trim($name) !== '' && $name !== 'untitled' ? [$source => $name] : [];
        $entry = ['ref' => "scene:$id", 'kind' => 'scene', 'name' => $id, 'labels' => $labels, 'origin' => ['key' => "scene:$id", 'state' => 'published']];
        foreach (self::objects($doc->node ?? null) as $node) {
            if (($node->kind ?? null) !== 'space') continue;
            if (is_string($node->set ?? null) && $node->set !== '') $entry['set'] = $node->set;
            if (is_string($node->spot ?? null) && $node->spot !== '') $entry['spot'] = $node->spot;
            break;
        }
        if (is_int($doc->version ?? null)) $entry['version'] = $doc->version;
        $placeholders = self::placeholders($doc);
        if ($placeholders !== []) $entry['placeholders'] = $placeholders;
        return $entry;
    }

    /**
     * A scene's placeholders, copied from the stored document (#504, #509): the slots the generated scene schema types as
     * a placeholder – the location as a `LocationPlaceholder` (an object), each `Holder` with a `fill` – in its order, their
     * `id`, `fill`, `label` and `preview` (a list). Nothing is judged here: the save's schema guard (ContentSchemas, generated
     * from the core's SceneDocument) refuses an unknown kind, the core's publish check the rest; the core's projection
     * (scene_document.rs `placeholders`) and this copy are held equal by the shared vectors book-entries.json.
     * @return list<ScenePlaceholder>
     */
    private static function placeholders(stdClass $doc): array
    {
        $out = [];
        $slots = array_merge(($doc->location ?? null) instanceof stdClass ? [$doc->location] : [], self::objects($doc->holder ?? null));
        foreach ($slots as $slot) {
            if (!isset($slot->fill)) continue;
            /** @var ScenePlaceholder $copy the stored slot (its shape guarded on save) */
            $copy = json_decode((string) json_encode($slot), true);
            $fill = ['kind' => $copy['fill']['kind']];
            if (($copy['fill']['ref'] ?? []) !== []) $fill['ref'] = $copy['fill']['ref'];
            if (isset($copy['fill']['parent'])) $fill['parent'] = $copy['fill']['parent'];
            $placeholder = ['id' => $copy['id'], 'fill' => $fill];
            if (($copy['label'] ?? []) !== []) $placeholder['label'] = $copy['label'];
            $preview = (array) ($copy['preview'] ?? []);
            if ($preview !== []) $placeholder['preview'] = array_values($preview);
            $out[] = $placeholder;
        }
        return $out;
    }

    /**
     * The toon's characters: its cast in season order, then the ones whose own toon it is; removed ones and ids without a
     * document left out
     * @param array<string, array{doc: mixed, state: OriginState}> $docs
     * @return list<string>
     */
    private function characters(string $toon, array $docs): array
    {
        $ids = [];
        $cast = $docs["cast:$toon"]['doc'] ?? null;
        $seasons = $cast instanceof stdClass && ($cast->seasons ?? null) instanceof stdClass ? get_object_vars($cast->seasons) : [];
        ksort($seasons, SORT_NUMERIC);
        foreach ($seasons as $entries) {
            foreach (self::objects($entries) as $entry) {
                if (is_string($entry->character ?? null)) $ids[$entry->character] = true;
            }
        }
        foreach ($docs as $key => ['doc' => $doc]) {
            if (str_starts_with($key, 'character:') && $doc instanceof stdClass && ($doc->own_toon ?? null) === $toon) $ids[substr($key, strlen('character:'))] = true;
        }
        return array_values(array_filter(array_map('strval', array_keys($ids)), function (string $id) use ($docs): bool {
            $doc = $docs["character:$id"]['doc'] ?? null;
            return $doc instanceof stdClass && ($doc->removed ?? false) !== true;
        }));
    }

    /** @return list<stdClass> */
    private static function objects(mixed $list): array
    {
        return is_array($list) ? array_values(array_filter($list, fn(mixed $v) => $v instanceof stdClass)) : [];
    }

    /** @return LocaleText a locale map's texts (nothing else counts; a plain string is no locale map) */
    private static function text(mixed $value): array
    {
        $out = [];
        if (!$value instanceof stdClass) return $out;
        foreach (get_object_vars($value) as $locale => $text) {
            if (is_string($text)) $out[(string) $locale] = $text;
        }
        return $out;
    }

    private static function catalogToon(string $data): ?string
    {
        $item = CatalogStore::reader($data);
        $toon = is_array($item) ? $item['toon'] ?? null : null;
        return is_string($toon) && $toon !== '' ? $toon : null;
    }

    /**
     * The document as JSON wants it: an empty label map stays an object
     * @param Book $book
     * @return array<string, mixed>
     */
    private static function wire(array $book): array
    {
        $book['entries'] = array_map(function (array $e): array {
            $e['labels'] = (object) $e['labels'];
            return $e;
        }, $book['entries']);
        return $book;
    }
}
