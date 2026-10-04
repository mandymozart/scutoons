<?php
declare(strict_types=1);

namespace Scutoons\Domain\Catalog;

use Scutoons\Infrastructure\Config\Settings;

/**
 * The toons QR codes can point to (table `toons`: id, name, redirectUrl – created by admins, agents/CATALOG.md; the
 * content of a toon is the creator's `toon:<id>` entity)
 */
final class Toons
{
    public function __construct(private readonly Settings $settings, private readonly CatalogStore $store)
    {
    }

    /** The seed file (config/toons.json) */
    public function path(): string
    {
        return $this->settings->path('TOONS_PATH');
    }

    /** @return list<array<string, mixed>> */
    public function all(): array
    {
        /** @var list<array<string, mixed>> */
        return array_values(array_filter($this->store->list('toons'), 'is_array'));
    }

    /** @return array<string, mixed>|null */
    public function find(string $id): ?array
    {
        $toon = $this->store->get('toons', $id);
        return is_array($toon) ? $toon : null;
    }
}
