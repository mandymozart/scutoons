<?php
declare(strict_types=1);

namespace Scutoons\Domain\Catalog;

use Scutoons\Infrastructure\Config\Settings;

/** Limited editions (config/editions.json): the physical products QR codes are printed for */
final class Editions
{
    /** @var list<array<string, mixed>>|null */
    private ?array $list = null;

    public function __construct(private readonly Settings $settings)
    {
    }

    public function path(): string
    {
        return $this->settings->path('EDITIONS_PATH');
    }

    /** @return list<array<string, mixed>> as stored (the public GET /api/editions/ list) */
    public function all(): array
    {
        if ($this->list === null) {
            $list = json_decode((string) @file_get_contents($this->path()), true);
            $this->list = is_array($list) ? array_values($list) : [];
        }
        return $this->list;
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
        foreach ($this->all() as $edition) {
            if (($edition['id'] ?? null) === $id) return $edition;
        }
        return null;
    }
}
