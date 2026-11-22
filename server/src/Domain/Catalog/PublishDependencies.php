<?php
declare(strict_types=1);

namespace Scutoons\Domain\Catalog;

use Scutoons\Domain\Content\EntityKey;
use Scutoons\Domain\Content\EntityStore;

/**
 * The one collector of every Publish (#803): what a set of documents needs that is still an unpublished draft, transitively
 * (a scene → the rigs and modules it plays; a rig → its instruments, presets, policy module; a preset → its instrument and
 * parent; an instrument → its modules). What names what is PublishOrder::refs; a factory module, a removal and a draft that
 * is the published document again are never added.
 */
final class PublishDependencies
{
    public function __construct(private readonly EntityStore $store, private readonly CatalogDocs $docs)
    {
    }

    /**
     * @param array<string, int|null> $versions key → the version that goes live (null: the newest)
     * @return array<string, string> each added key → the key that needs it (the first one found)
     */
    public function of(array $versions): array
    {
        $factory = array_flip($this->docs->factoryModuleKeys());
        $seen = array_fill_keys(array_keys($versions), true);
        $queue = [];
        foreach ($versions as $key => $version) {
            $row = $version !== null ? $this->store->byId($version) : $this->store->latest($key);
            if ($row && $row['entity_key'] === $key) $queue[] = [$key, $row];
        }
        $added = [];
        while ($queue !== []) {
            [$key, $row] = array_shift($queue);
            foreach (PublishOrder::refs(EntityKey::parse($key), json_decode((string) $row['json'], true)) as $ref) {
                if (isset($seen[$ref])) continue;
                $seen[$ref] = true;
                if (isset($factory[$ref])) continue;
                $draft = $this->draft($ref);
                if ($draft === null) continue;
                $added[$ref] = $key;
                $queue[] = [$ref, $draft];
            }
        }
        return $added;
    }

    /**
     * The newest version of a key when readers do not see it yet (null: none stored, the published one, the published
     * document again, or a removal – a dependency is never taken away by publishing what needs it)
     * @return array<string, mixed>|null
     */
    private function draft(string $key): ?array
    {
        $latest = $this->store->latest($key);
        if ($latest === null) return null;
        $published = $this->store->published($key);
        if ($published !== null && ((int) $latest['id'] <= (int) $published['id'] || $latest['json'] === $published['json'])) return null;
        return CatalogDocs::isTombstone(json_decode((string) $latest['json'], true)) ? null : $latest;
    }
}
