<?php
declare(strict_types=1);

namespace Scutoons\Domain\Catalog;

use Scutoons\Infrastructure\Config\Settings;

/** The toons QR codes can point to (config/toons.json: id, name, redirectUrl) */
final class Toons
{
    public function __construct(private readonly Settings $settings)
    {
    }

    public function path(): string
    {
        return $this->settings->path('TOONS_PATH');
    }

    /** @return list<array<string, mixed>> */
    public function all(): array
    {
        $list = json_decode((string) @file_get_contents($this->path()), true);
        return is_array($list) ? array_values($list) : [];
    }

    /** @return array<string, mixed>|null */
    public function find(string $id): ?array
    {
        foreach ($this->all() as $toon) {
            if (($toon['id'] ?? null) === $id) return $toon;
        }
        return null;
    }
}
