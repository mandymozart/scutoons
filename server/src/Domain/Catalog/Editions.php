<?php
declare(strict_types=1);

namespace Scutoons\Domain\Catalog;

use Scutoons\Infrastructure\Config\Settings;

/** Limited editions (table catalog_editions, agents/CATALOG.md): the physical products QR codes are printed for */
final class Editions
{
    public function __construct(private readonly Settings $settings, private readonly CatalogStore $store)
    {
    }

    /** The seed file (config/editions.json) */
    public function path(): string
    {
        return $this->settings->path('EDITIONS_PATH');
    }

    /** @return list<array<string, mixed>> in their order (the public GET /api/editions/ list) */
    public function all(): array
    {
        /** @var list<array<string, mixed>> */
        return array_values(array_filter($this->store->list('editions'), 'is_array'));
    }

    /** @return list<array<string, mixed>> */
    public function forToon(string $toon): array
    {
        return array_values(array_filter($this->all(), fn(array $edition) => ($edition['toon'] ?? null) === $toon));
    }

    /** @return array<string, mixed>|null */
    public function find(?string $id): ?array
    {
        if ($id === null || $id === '') return null;
        $edition = $this->store->get('editions', $id);
        return is_array($edition) ? $edition : null;
    }
}
