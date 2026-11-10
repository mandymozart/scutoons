<?php
declare(strict_types=1);

namespace Scutoons\Domain\Addresses;

use Scutoons\Domain\Catalog\CatalogStore;
use Scutoons\Domain\Library\Library;
use Scutoons\Generated\AddressBookTypes;
use Scutoons\Infrastructure\Cache\Cache;
use Scutoons\Infrastructure\Database\Database;
use stdClass;

/**
 * One toon's address book (#477; the shape: server/schema/api/address-book.schema.json, its PHP types AddressBookTypes):
 * every addressable thing of the toon's catalog, read by the creator's VN events and the deck editor in one typeahead each.
 *
 * - **characters** in the toon's cast (every season) or with `own_toon` = the toon: the character, each persona, each look
 *   with the moods it has art for – `character:<id>`, `character:<id>.<persona>`, `character:<id>.<persona>.<look>`
 *   (address.rs `Target`, SHOW.md §8)
 * - **locations**: the world map's places whose `toons` hold the toon – `location:<place>` (SHOW.md §8)
 * - **items**: the catalog's items of the toon and the shared ones (no toon) – `item:<id>`
 * - **chapters**: the toon's chapters – `chapter:<number>`
 * Not in it yet (#477, the questions there): a chapter's groups and the story variables (no form in address.rs's grammar),
 * deck scenes and sets (no toon in the catalog). Removed characters and places (tombstones) are left out.
 *
 * Each document's newest version counts (drafts included: the creator and the editor author against them); `origin.state`
 * says whether that version is the published one. Built on request and cached under the version ids it was built from
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
    private const JSON_FLAGS = JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_THROW_ON_ERROR;

    public function __construct(private readonly Database $db, private readonly CatalogStore $catalog, private readonly Cache $cache)
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
             WHERE locale IS NULL AND ((kind IN ('toon', 'cast', 'chapter') AND toon = ?) OR kind IN ('character', 'place'))
             GROUP BY entity_key ORDER BY entity_key",
            [$toon]
        );
        $versions = [];
        foreach ($rows as $r) $versions[(string) $r['entity_key']] = [(int) $r['latest'], $r['published'] !== null ? (int) $r['published'] : null];
        if (!isset($versions["toon:$toon"])) return null;
        $items = array_values(array_filter($this->catalog->rows('items'), fn(array $r) => in_array(self::itemToon($r['data']), [$toon, null], true)));
        $version = substr(sha1(AddressBookTypes::FORMAT . json_encode([$versions, array_map(fn(array $r) => [$r['id'], sha1($r['data'])], $items)])), 0, 20);
        $key = "address-book:$toon:$version";
        $cached = $this->cache->get($key);
        if ($cached !== null) return ['version' => $version, 'json' => $cached];
        $book = $this->compose($toon, $version, $this->documents($versions), $items);
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
     * @param array<string, array{doc: mixed, state: OriginState}> $docs
     * @param list<array{id: string, position: int, data: string, updated_at: ?string, updated_by: ?string}> $items
     * @return Book
     */
    private function compose(string $toon, string $version, array $docs, array $items): array
    {
        $toonDoc = $docs["toon:$toon"]['doc'];
        [$source] = Library::languages($toonDoc instanceof stdClass ? $toonDoc : null, []);
        $entries = [];
        foreach ($this->characters($toon, $docs) as $id) {
            ['doc' => $doc, 'state' => $state] = $docs["character:$id"];
            $origin = ['key' => "character:$id", 'state' => $state];
            $entries[] = ['address' => "character:$id", 'kind' => 'character', 'name' => $id, 'labels' => self::text($doc->name ?? null), 'origin' => $origin];
            foreach (self::objects($doc->personas ?? null) as $persona) {
                if (!is_string($persona->id ?? null) || $persona->id === '') continue;
                $p = "character:$id.$persona->id";
                $entries[] = ['address' => $p, 'kind' => 'persona', 'name' => $persona->id, 'parent' => "character:$id", 'labels' => self::text($persona->label ?? null), 'origin' => $origin];
                foreach (self::objects($persona->looks ?? null) as $look) {
                    if (!is_string($look->id ?? null) || $look->id === '') continue;
                    $moods = ($look->images ?? null) instanceof stdClass ? array_map('strval', array_keys(get_object_vars($look->images))) : [];
                    $entries[] = ['address' => "$p.$look->id", 'kind' => 'look', 'name' => $look->id, 'parent' => $p, 'labels' => self::text($look->label ?? null), 'origin' => $origin, 'moods' => $moods];
                }
            }
        }
        foreach ($docs as $key => ['doc' => $doc, 'state' => $state]) {
            if (!str_starts_with($key, 'place:') || !$doc instanceof stdClass || ($doc->removed ?? false) === true) continue;
            if (!in_array($toon, is_array($doc->toons ?? null) ? $doc->toons : [], true)) continue;
            $id = substr($key, strlen('place:'));
            $entries[] = ['address' => "location:$id", 'kind' => 'location', 'name' => $id, 'labels' => self::text($doc->name ?? null), 'origin' => ['key' => $key, 'state' => $state]];
        }
        foreach ($items as $row) {
            $item = CatalogStore::reader($row['data'], false);
            $entries[] = ['address' => "item:{$row['id']}", 'kind' => 'item', 'name' => $row['id'], 'labels' => self::text($item instanceof stdClass ? $item->name ?? null : null), 'origin' => ['key' => "item:{$row['id']}", 'state' => 'published']];
        }
        $chapters = [];
        foreach ($docs as $key => ['doc' => $doc, 'state' => $state]) {
            if (preg_match('#^chapter:' . preg_quote($toon, '#') . ':([0-9]+)$#', $key, $m) !== 1) continue;
            $chapters[(int) $m[1]] = ['address' => "chapter:$m[1]", 'kind' => 'chapter', 'name' => $m[1], 'labels' => self::text($doc->title ?? null), 'origin' => ['key' => $key, 'state' => $state]];
        }
        ksort($chapters);
        return ['address_book' => AddressBookTypes::FORMAT, 'toon' => $toon, 'version' => $version, 'source' => $source, 'entries' => [...$entries, ...array_values($chapters)]];
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

    private static function itemToon(string $data): ?string
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
