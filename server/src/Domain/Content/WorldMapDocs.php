<?php
declare(strict_types=1);

namespace Scutoons\Domain\Content;

use Scutoons\Domain\Library\Snapshot;
use stdClass;

/**
 * The world map as many documents (DRAFT, agents/MAP-DOCUMENTS.md): `map` = its settings { width, height, layers },
 * `place:<id>`, `route:<id>` – a CollectionDocs::MAP. Everything that reads the map (the reader's /library/map, the old
 * bundle lists, the flow index) works on ONE composed map in the old shape { width, height, layers, locations, routes }.
 */
final class WorldMapDocs
{
    /** the kinds a composed map is made of */
    public const KINDS = ['map', 'place', 'route'];

    /** The composed map of a snapshot (published or newest versions), null when it has no `map` document */
    public static function fromSnapshot(Snapshot $snapshot): ?stdClass
    {
        $map = CollectionDocs::fromSnapshot(CollectionDocs::MAP, $snapshot);
        if ($map === null) return null;
        $map->width ??= 0;
        $map->height ??= 0;
        if (!is_array($map->layers ?? null)) $map->layers = [];
        return $map;
    }

    /**
     * The composed map as arrays from entity key → document arrays (FlowIndex), null without a `map` document
     * @param array<string, array<mixed>> $docs
     * @return array<string, mixed>|null
     */
    public static function fromArrays(array $docs): ?array
    {
        return CollectionDocs::fromArrays(CollectionDocs::MAP, $docs);
    }
}
