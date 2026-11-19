<?php
declare(strict_types=1);

namespace Scutoons\Domain\Library;

use Scutoons\Domain\Catalog\CatalogStore;
use Scutoons\Domain\Content\EntityStore;

/**
 * The catalog's device presets as a deck reads them: `[{ version, document }]` – deck-space device_preset.rs `Listed`
 * (decoded with objects). `version` = the published `device-preset:<id>` version's row id (it only grows, so a deck that
 * names an older one is offered the newer); an entry an admin wrote without a document is version 0. The factory presets
 * are not here: they ship with the deck.
 */
final class DevicePresets
{
    public function __construct(private readonly CatalogStore $store, private readonly EntityStore $entities)
    {
    }

    /** @return list<array{version: int, document: mixed}> */
    public function listed(): array
    {
        $out = [];
        foreach ($this->store->rows('device-presets') as $row) {
            $published = $this->entities->published("device-preset:{$row['id']}");
            $out[] = ['version' => $published !== null ? (int) $published['id'] : 0, 'document' => CatalogStore::reader($row['data'], false)];
        }
        return $out;
    }
}
