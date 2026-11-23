<?php
declare(strict_types=1);

namespace Scutoons\Domain\Library;

use Scutoons\Domain\Catalog\CatalogDocs;
use Scutoons\Domain\Catalog\CatalogStore;
use Scutoons\Domain\Content\EntityStore;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Time;

/**
 * An XR experience as the reader's `/xr/<id>` reads it (research/XR.md §4.4): the published `experience:<id>` (its catalog
 * row), decoded with objects so OSCT's configuration keeps its `{}`. One row is read, not the kind: documents are large.
 */
final class Experiences
{
    public function __construct(private readonly Database $db, private readonly EntityStore $entities)
    {
    }

    /**
     * `{ experience, updatedAt }`; $draft (creator preview): the newest version; null = none (or removed)
     * @return array{experience: mixed, updatedAt: ?string}|null
     */
    public function get(string $id, bool $draft): ?array
    {
        if ($draft) {
            $version = $this->entities->latest("experience:$id");
            if ($version === null || CatalogDocs::isTombstone(json_decode((string) $version['json'], true))) return null;
            return ['experience' => CatalogStore::reader((string) $version['json'], false), 'updatedAt' => Time::iso($version['created_at'])];
        }
        $row = $this->db->one('SELECT data, updated_at FROM ' . CatalogStore::table('experiences') . ' WHERE id = ?', [$id]);
        if ($row === null) return null;
        return ['experience' => CatalogStore::reader((string) $row['data'], false), 'updatedAt' => Time::iso($row['updated_at'])];
    }
}
