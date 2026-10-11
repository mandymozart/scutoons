<?php
declare(strict_types=1);

namespace Scutoons\Domain\Content;

use Scutoons\Domain\Library\Snapshot;
use stdClass;

/**
 * The world map as many documents (DRAFT, agents/MAP-DOCUMENTS.md; the TS twin is @scutoons/content map-docs.ts – keep
 * both in step): `map` = the map's own settings { width, height, layers }, `place:<id>` = one place without its id,
 * `route:<id>` = one route. Everything that reads the map (the reader's /library/map, the old bundle lists, the flow
 * index) works on ONE composed map in the old shape { width, height, layers, locations, routes }.
 *
 * A document with `removed: true` is a tombstone (the place / route was taken off the map; its id stays taken) and is
 * left out. Order: creation order of the documents (the first version's id), so adding a place touches no other
 * document. Accepted warnings stay in their documents (only the creator needs them – the TS composer collects them).
 */
final class WorldMapDocs
{
    /** the kinds a composed map is made of */
    public const KINDS = ['map', 'place', 'route'];

    /**
     * The composed map of a snapshot (published or newest versions), null when it has no `map` document
     */
    public static function fromSnapshot(Snapshot $snapshot): ?stdClass
    {
        $settings = $snapshot->doc('map');
        if (!$settings instanceof stdClass) return null;
        $parts = function (string $kind) use ($snapshot): array {
            $out = [];
            foreach ($snapshot->createdOrder("$kind:") as $key) $out[substr($key, strlen($kind) + 1)] = $snapshot->doc($key);
            return $out;
        };
        return self::compose($settings, $parts('place'), $parts('route'));
    }

    /**
     * The composed map of decoded documents (objects as stdClass)
     * @param array<string, mixed> $places id → document, in map order
     * @param array<string, mixed> $routes id → document, in map order
     */
    public static function compose(stdClass $settings, array $places, array $routes): stdClass
    {
        $list = function (array $docs): array {
            $out = [];
            foreach ($docs as $id => $doc) {
                if (!$doc instanceof stdClass || ($doc->removed ?? false) === true) continue;
                $entry = new stdClass();
                $entry->id = (string) $id;
                foreach (get_object_vars($doc) as $k => $v) {
                    if ($k !== 'removed' && $k !== 'accepted_warnings' && $k !== 'id') $entry->{$k} = $v;
                }
                $out[] = $entry;
            }
            return $out;
        };
        $map = new stdClass();
        $map->width = $settings->width ?? 0;
        $map->height = $settings->height ?? 0;
        $map->layers = is_array($settings->layers ?? null) ? $settings->layers : [];
        $map->locations = $list($places);
        $map->routes = $list($routes);
        return $map;
    }

    /**
     * The composed map as arrays from entity key → document arrays (FlowIndex: every key once, sorted by key), null
     * without a `map` document
     * @param array<string, array<mixed>> $docs
     * @return array<string, mixed>|null
     */
    public static function fromArrays(array $docs): ?array
    {
        if (!is_array($docs['map'] ?? null)) return null;
        $settings = json_decode((string) json_encode($docs['map']));
        if (!$settings instanceof stdClass) $settings = new stdClass();
        $parts = function (string $kind) use ($docs): array {
            $out = [];
            foreach ($docs as $key => $doc) {
                if (str_starts_with($key, "$kind:")) $out[substr($key, strlen($kind) + 1)] = json_decode((string) json_encode($doc));
            }
            return $out;
        };
        $map = json_decode((string) json_encode(self::compose($settings, $parts('place'), $parts('route'))), true);
        return is_array($map) ? $map : null;
    }
}
