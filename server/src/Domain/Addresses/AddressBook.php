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
 * - **chapters**: the toon's chapters – `chapter:<number>`, each followed by its **groups** – `chapter:<number>.<group>`; references
 * - **products**: the shop's products of the toon and the shared ones – `product:<id>`; references
 * - **toons**: every toon, this one first – `toon:<id>`; references (a jump's other toon; XR targets have no catalog to list)
 * - **scenes** (#487, research/SCENES.md §3 / §8): the deck scenes whose published version names the toon in its `toons` –
 *   `scene:<id>`, by id; addresses (a deck plays one: a show cell's piece). Only the published version counts – a draft never
 *   leaves the editor – and it gives the label (its stage's name, under the source language), the set and spot of its
 *   `space` node where it says them, and its stamped `version`. A scene with no toons is in no book (reachable by its id).
 *   `scene` is the deck scene only: no chapter group, screenplay heading or sandbox is ever listed as one (sandboxes are
 *   files, never in entity_versions)
 * - **variables**: every story variable of the one save game (the flow index's, every toon may set any) –
 *   `variable:<scope>.<name>`; references
 * The group and variable forms are proposals the owner has not confirmed (#477, slice B): cheap to change, here and in the
 * schema's AddressKind only. A group's `name` is its label exactly as an event stores it; its reference word is a safe form
 * of the label (letters, digits, `_` and `-`; anything else becomes `-`, a second label with the same form gets `-2` …). A
 * variable's `name` is the variable as written in this toon (`coins`, `global.coins`, `toptoy.coins`).
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
        $rows = $this->db->all(
            "SELECT entity_key, MAX(id) AS latest, MAX(CASE WHEN status = 'published' THEN id END) AS published FROM entity_versions
             WHERE locale IS NULL AND ((kind IN ('cast', 'chapter', 'season') AND toon = ?) OR kind IN ('toon', 'character', 'place', 'region'))
             GROUP BY entity_key ORDER BY entity_key",
            [$toon]
        );
        $versions = [];
        foreach ($rows as $r) $versions[(string) $r['entity_key']] = [(int) $r['latest'], $r['published'] !== null ? (int) $r['published'] : null];
        if (!isset($versions["toon:$toon"])) return null;
        $mine = fn(array $r) => in_array(self::catalogToon($r['data']), [$toon, null], true);
        $items = array_values(array_filter($this->catalog->rows('items'), $mine));
        $products = array_values(array_filter($this->catalog->rows('products'), $mine));
        $variables = $this->variables();
        $scenes = $this->scenes($toon);
        $rows = fn(array $list) => array_map(fn(array $r) => [$r['id'], sha1($r['data'])], $list);
        $version = substr(sha1(AddressBookTypes::FORMAT . json_encode([$versions, $rows($items), $rows($products), $variables, array_map(fn(array $s) => $s['row'], $scenes)])), 0, 20);
        $key = "address-book:$toon:$version";
        $cached = $this->cache->get($key);
        if ($cached !== null) return ['version' => $version, 'json' => $cached];
        $book = $this->compose($toon, $version, $this->documents($versions), $items, $products, $variables, $scenes);
        $json = json_encode(self::wire($book), self::JSON_FLAGS);
        $this->cache->set($key, $json, self::CACHE_TTL);
        return ['version' => $version, 'json' => $json];
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
     * The published deck scenes whose `toons` name the toon, by id (a draft never leaves the editor; an empty `toons` is in no book)
     * @return array<string, array{row: int, doc: stdClass}> scene id → the published version's row id and document
     */
    private function scenes(string $toon): array
    {
        $out = [];
        foreach ($this->db->all("SELECT id, entity_key, json FROM entity_versions WHERE kind = 'scene' AND status = 'published' AND locale IS NULL ORDER BY entity_key") as $r) {
            $doc = json_decode((string) $r['json']);
            if (!$doc instanceof stdClass || !in_array($toon, is_array($doc->toons ?? null) ? $doc->toons : [], true)) continue;
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
            $words = [];
            foreach (self::objects($doc instanceof stdClass ? $doc->groups ?? null : null) as $group) {
                $label = $group->label ?? null;
                if (!is_string($label) || $label === '' || isset($words[$label])) continue;
                $word = self::word($label, $words);
                $words[$label] = $word;
                $chapters[(int) $m[1]][] = ['ref' => "$chapter.$word", 'kind' => 'group', 'name' => $label, 'parent' => $chapter, 'labels' => [], 'origin' => $origin];
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
        return $entry;
    }

    /**
     * A group label as a reference word (proposal, #477): letters, digits, `_` and `-` stay, every other run becomes `-`
     * (the grammar's word may not hold `/ : . @ #` or white space); a form another label of the chapter already has gets `-2`, `-3` …
     * @param array<string, string> $taken label → its word, this chapter's so far
     */
    private static function word(string $label, array $taken): string
    {
        $base = trim((string) preg_replace('/[^\p{L}\p{N}_-]+/u', '-', $label), '-');
        if ($base === '') $base = 'group';
        $word = $base;
        for ($n = 2; in_array($word, $taken, true); $n++) $word = "$base-$n";
        return $word;
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
