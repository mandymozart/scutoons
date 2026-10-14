<?php
declare(strict_types=1);

namespace Scutoons\Domain\Catalog;

use Scutoons\Domain\Avatar\AvatarSlots;
use Scutoons\Domain\Reader\ReaderStats;
use Scutoons\Infrastructure\Config\Settings;

/**
 * The item catalog (agents/ITEMS.md "Item definition"): the tables catalog_items / catalog_recipes / catalog_albums
 * (CatalogStore, agents/CATALOG.md) + the slot types and reader stats from code (AvatarSlots, ReaderStats), served by
 * GET /api/items/ in the shape config/items.json had – and helpers over it. Editions are looked up through Editions.
 */
final class Catalog
{
    /** @var array<string, mixed>|null */
    private ?array $data = null;
    /** @var array<string, array<string, mixed>>|null */
    private ?array $items = null;
    /** the store's generation the data was read at (a write in the same request reads again) */
    private int $generation = -1;

    public function __construct(private readonly Settings $settings, private readonly Editions $editions, private readonly CatalogStore $store)
    {
    }

    /**
     * @return array<string, mixed> the whole catalog as served by GET /api/items/: { version, slots, items, recipes,
     *   albums, stats, home_scenes } (the order of the old file; home scenes – research/CAVE.md)
     */
    public function data(): array
    {
        if ($this->data === null || $this->generation !== $this->store->generation()) {
            $this->items = null;
            $this->generation = $this->store->generation();
            $meta = $this->store->get('meta', 'items');
            $this->data = [
                'version' => is_array($meta) ? ($meta['version'] ?? 1) : 1,
                'slots' => AvatarSlots::ALL,
                'items' => $this->store->list('items'),
                'recipes' => $this->store->list('recipes'),
                'albums' => $this->store->list('albums'),
                'stats' => ReaderStats::DEFINITIONS,
                'home_scenes' => $this->store->list('home-scenes'),
            ];
        }
        return $this->data;
    }

    /** @return array<string, array<string, mixed>> id => item */
    public function items(): array
    {
        $data = $this->data();
        if ($this->items === null) {
            $this->items = [];
            foreach ($data['items'] as $item) $this->items[$item['id']] = $item;
        }
        return $this->items;
    }

    /** @return array<string, mixed>|null */
    public function item(string $id): ?array
    {
        return $this->items()[$id] ?? null;
    }

    /** @return list<string> the avatar slot types (code: AvatarSlots) */
    public function slots(): array
    {
        return AvatarSlots::ALL;
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

    /**
     * Can a reader gift this item to another reader (agents/ITEMS.md "Ownership, claims, gifting")? The item's
     * `giftable`, else true – except starters / the basic dress (everyone has them) and membership / access-only items.
     * @param array<string, mixed> $item
     */
    public static function giftable(array $item): bool
    {
        if (is_bool($item['giftable'] ?? null)) return $item['giftable'];
        return empty($item['starter']) && empty($item['default_worn']) && !in_array($item['category'] ?? null, ['membership', 'access'], true);
    }

    /** @return list<array<string, mixed>> starter items (every new inventory gets one) */
    public function starters(): array
    {
        return array_values(array_filter($this->items(), fn($item) => !empty($item['starter'])));
    }

    /**
     * The tools whose scan_object skill grants this item and their smallest cooldown (seconds); null = no tool grants it.
     * @return array{tools: list<string>, cooldown: int, per_day: int}|null
     */
    public function scanTools(string $itemId): ?array
    {
        $tools = [];
        $cooldown = null;
        $perDay = 0; // 0 = no daily limit; the most generous camera wins (a tool without `per_day` lifts it)
        $limited = true;
        foreach ($this->items() as $item) {
            foreach ($item['skills'] ?? [] as $skill) {
                // `grants`, or one of `map[].grants` (a later multi-label camera, agents/ITEMS.md "Substances")
                $grants = [$skill['grants'] ?? null, ...array_map(fn($m) => is_array($m) ? ($m['grants'] ?? null) : null, is_array($skill['map'] ?? null) ? $skill['map'] : [])];
                if (($skill['type'] ?? '') === 'scan_object' && in_array($itemId, $grants, true)) {
                    $tools[] = (string) $item['id'];
                    $seconds = (int) ($skill['cooldown_s'] ?? 0);
                    $cooldown = $cooldown === null ? $seconds : min($cooldown, $seconds);
                    $day = (int) ($skill['per_day'] ?? 0);
                    if ($day > 0) $perDay = max($perDay, $day); else $limited = false;
                }
            }
        }
        return $tools ? ['tools' => $tools, 'cooldown' => (int) $cooldown, 'per_day' => $limited ? $perDay : 0] : null;
    }

    /** @return array<string, mixed>|null the edition (catalog_editions) */
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
