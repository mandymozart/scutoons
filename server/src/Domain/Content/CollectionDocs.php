<?php
declare(strict_types=1);

namespace Scutoons\Domain\Content;

use Scutoons\Domain\Library\Snapshot;
use Scutoons\Infrastructure\Database\Database;
use stdClass;

/**
 * A document made of many documents (DRAFT, agents/MAP-DOCUMENTS.md; the TS twin is @scutoons/content
 * collection-docs.ts – keep both in step): an anchor document with the collection's own fields plus one document per
 * entry of its lists (`<kind>:<id>`, the entry without its id). Everything that reads it gets ONE composed document in
 * the old shape:
 *
 *   MAP          map        + place:<id> → locations, route:<id> → routes      (the world map)
 *   CHARACTERS   characters + character:<id> → characters                       (the character registry)
 *
 * A document with `removed: true` is a tombstone (the entry left; its id stays taken) and is left out. Order: creation
 * order of the documents (the first version's id). Accepted warnings stay in their documents (only the creator needs
 * them – the TS composer collects them).
 */
final class CollectionDocs
{
    /** @var array{key: string, parts: array<string, string>} anchor key + part kind → list field */
    public const MAP = ['key' => 'map', 'parts' => ['place' => 'locations', 'route' => 'routes']];
    /** @var array{key: string, parts: array<string, string>} */
    public const CHARACTERS = ['key' => 'characters', 'parts' => ['character' => 'characters']];

    /**
     * every kind a collection is stored as (the anchor first)
     * @param array{key: string, parts: array<string, string>} $spec
     * @return list<string>
     */
    public static function kinds(array $spec): array
    {
        return [$spec['key'], ...array_keys($spec['parts'])];
    }

    /**
     * The composed document of a snapshot (published or newest versions), null without the anchor document
     * @param array{key: string, parts: array<string, string>} $spec
     */
    public static function fromSnapshot(array $spec, Snapshot $snapshot): ?stdClass
    {
        $settings = $snapshot->doc($spec['key']);
        if (!$settings instanceof stdClass) return null;
        $parts = [];
        foreach (array_keys($spec['parts']) as $kind) {
            $parts[$kind] = [];
            foreach ($snapshot->createdOrder("$kind:") as $key) $parts[$kind][substr($key, strlen($kind) + 1)] = $snapshot->doc($key);
        }
        return self::compose($spec, $settings, $parts);
    }

    /**
     * The composed document of the PUBLISHED versions, straight from the store (for code without a snapshot:
     * CharacterUnlocks), null without a published anchor
     * @param array{key: string, parts: array<string, string>} $spec
     */
    public static function published(array $spec, Database $db): ?stdClass
    {
        return self::fromSnapshot($spec, Snapshot::kinds($db, self::kinds($spec), false));
    }

    /**
     * The composed document of decoded documents (objects as stdClass)
     * @param array{key: string, parts: array<string, string>} $spec
     * @param array<string, array<string, mixed>> $parts part kind → (id → document), in order
     */
    public static function compose(array $spec, stdClass $settings, array $parts): stdClass
    {
        $out = new stdClass();
        foreach (get_object_vars($settings) as $k => $v) {
            if ($k !== 'accepted_warnings' && !in_array($k, $spec['parts'], true)) $out->{$k} = $v;
        }
        foreach ($spec['parts'] as $kind => $field) {
            $list = [];
            foreach ($parts[$kind] ?? [] as $id => $doc) {
                if (!$doc instanceof stdClass || ($doc->removed ?? false) === true) continue;
                $entry = new stdClass();
                $entry->id = (string) $id;
                foreach (get_object_vars($doc) as $k => $v) {
                    if ($k !== 'removed' && $k !== 'accepted_warnings' && $k !== 'id') $entry->{$k} = $v;
                }
                $list[] = $entry;
            }
            $out->{$field} = $list;
        }
        return $out;
    }

    /**
     * The composed document as arrays from entity key → document arrays (FlowIndex), null without the anchor
     * @param array{key: string, parts: array<string, string>} $spec
     * @param array<string, array<mixed>> $docs
     * @return array<string, mixed>|null
     */
    public static function fromArrays(array $spec, array $docs): ?array
    {
        if (!is_array($docs[$spec['key']] ?? null)) return null;
        $settings = json_decode((string) json_encode($docs[$spec['key']]));
        if (!$settings instanceof stdClass) $settings = new stdClass();
        $parts = [];
        foreach (array_keys($spec['parts']) as $kind) {
            $parts[$kind] = [];
            foreach ($docs as $key => $doc) {
                if (str_starts_with($key, "$kind:")) $parts[$kind][substr($key, strlen($kind) + 1)] = json_decode((string) json_encode($doc));
            }
        }
        $composed = json_decode((string) json_encode(self::compose($spec, $settings, $parts)), true);
        return is_array($composed) ? $composed : null;
    }

    /**
     * An interchange file of the whole collection (`map.json`, `characters.json` – the old one-document shape) as its
     * documents: the anchor = everything but the lists, then `<kind>:<id>` per entry in the file's order. An entry
     * without a valid id is left out.
     * @param array{key: string, parts: array<string, string>} $spec
     * @return array<string, mixed> key → document
     */
    public static function split(array $spec, mixed $composed): array
    {
        if (!$composed instanceof stdClass) return [$spec['key'] => $composed];
        $settings = clone $composed;
        foreach ($spec['parts'] as $field) unset($settings->{$field});
        $out = [$spec['key'] => $settings];
        foreach ($spec['parts'] as $kind => $field) {
            foreach (is_array($composed->{$field} ?? null) ? $composed->{$field} : [] as $entry) {
                if (!$entry instanceof stdClass || !is_string($entry->id ?? null) || !preg_match(EntityKey::ID_PATTERN, $entry->id)) continue;
                $doc = clone $entry;
                unset($doc->id);
                $out["$kind:$entry->id"] = $doc;
            }
        }
        return $out;
    }

    /**
     * The collection an anchor key belongs to (`map`, `characters`), null for any other key
     * @return array{key: string, parts: array<string, string>}|null
     */
    public static function ofAnchor(string $key): ?array
    {
        foreach ([self::MAP, self::CHARACTERS] as $spec) if ($spec['key'] === $key) return $spec;
        return null;
    }

    /**
     * The collection a part kind belongs to (`place` → MAP), null for any other kind
     * @return array{key: string, parts: array<string, string>}|null
     */
    public static function ofPart(string $kind): ?array
    {
        foreach ([self::MAP, self::CHARACTERS] as $spec) if (isset($spec['parts'][$kind])) return $spec;
        return null;
    }
}
