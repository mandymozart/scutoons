<?php
declare(strict_types=1);

namespace Scutoons\Domain\Catalog;

use RuntimeException;
use Scutoons\Infrastructure\Config\Settings;

/**
 * The item catalog (config/items.json, agents/ITEMS.md "Item definition") – served as-is by GET /api/items/ – and
 * helpers over it. Editions (config/editions.json) are looked up through Editions.
 */
final class Catalog
{
    /** @var array<string, mixed>|null */
    private ?array $data = null;
    /** @var array<string, array<string, mixed>>|null */
    private ?array $items = null;

    public function __construct(private readonly Settings $settings, private readonly Editions $editions)
    {
    }

    public function path(): string
    {
        return $this->settings->path('ITEMS_PATH');
    }

    /** @return array<string, mixed> the whole catalog as served by GET /api/items/ */
    public function data(): array
    {
        if ($this->data === null) {
            $data = json_decode((string) @file_get_contents($this->path()), true);
            if (!is_array($data) || !isset($data['items'])) throw new RuntimeException('items.json is invalid');
            $this->data = $data;
        }
        return $this->data;
    }

    /** @return array<string, array<string, mixed>> id => item */
    public function items(): array
    {
        if ($this->items === null) {
            $this->items = [];
            foreach ($this->data()['items'] as $item) $this->items[$item['id']] = $item;
        }
        return $this->items;
    }

    /** @return array<string, mixed>|null */
    public function item(string $id): ?array
    {
        return $this->items()[$id] ?? null;
    }

    /** @return list<string> */
    public function slots(): array
    {
        return $this->data()['slots'] ?? [];
    }

    /** @return array<string, mixed>|null */
    public function recipe(string $id): ?array
    {
        foreach ($this->data()['recipes'] ?? [] as $recipe) {
            if ($recipe['id'] === $id) return $recipe;
        }
        return null;
    }

    /**
     * @param array<string, mixed> $item
     * @return list<string> variant ids of an item ([] = no variants)
     */
    public function variantIds(array $item): array
    {
        return array_values(array_map(fn($v) => (string) $v['id'], $item['variants'] ?? []));
    }

    /**
     * @param array<string, mixed> $item
     * @return array<string, mixed>|null
     */
    public function variant(array $item, ?string $variantId): ?array
    {
        foreach ($item['variants'] ?? [] as $variant) {
            if ($variant['id'] === $variantId) return $variant;
        }
        return null;
    }

    /** @return list<array<string, mixed>> catalog items that belong to an edition (the claim options of its codes) */
    public function itemsForEdition(string $editionId): array
    {
        return array_values(array_filter($this->items(), fn($item) => ($item['edition_id'] ?? null) === $editionId));
    }

    /** @return list<array<string, mixed>> starter items (every new inventory gets one) */
    public function starters(): array
    {
        return array_values(array_filter($this->items(), fn($item) => !empty($item['starter'])));
    }

    /**
     * The tools whose scan_object skill grants this item and their smallest cooldown (seconds); null = no tool grants it.
     * @return array{tools: list<string>, cooldown: int}|null
     */
    public function scanTools(string $itemId): ?array
    {
        $tools = [];
        $cooldown = null;
        foreach ($this->items() as $item) {
            foreach ($item['skills'] ?? [] as $skill) {
                // `grants`, or one of `map[].grants` (a later multi-label camera, agents/ITEMS.md "Substances")
                $grants = [$skill['grants'] ?? null, ...array_map(fn($m) => is_array($m) ? ($m['grants'] ?? null) : null, is_array($skill['map'] ?? null) ? $skill['map'] : [])];
                if (($skill['type'] ?? '') === 'scan_object' && in_array($itemId, $grants, true)) {
                    $tools[] = (string) $item['id'];
                    $seconds = (int) ($skill['cooldown_s'] ?? 0);
                    $cooldown = $cooldown === null ? $seconds : min($cooldown, $seconds);
                }
            }
        }
        return $tools ? ['tools' => $tools, 'cooldown' => (int) $cooldown] : null;
    }

    /** @return array<string, mixed>|null the edition from editions.json */
    public function edition(?string $id): ?array
    {
        return $this->editions->find($id);
    }

    /** Localised text of a { de, en } map */
    public static function text(mixed $map, string $locale): string
    {
        if (!is_array($map)) return is_scalar($map) ? (string) $map : '';
        $first = reset($map);
        return (string) ($map[$locale] ?? $map['en'] ?? ($first === false ? '' : $first));
    }

    /** Public URL of a CDN path (icons) */
    public function cdnUrl(?string $path): ?string
    {
        if ($path === null || $path === '') return null;
        return rtrim($this->settings->get('CDN_URL'), '/') . '/' . ltrim($path, '/');
    }
}
