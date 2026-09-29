<?php
declare(strict_types=1);

/** The item catalog (api/items/items.json, ITEMS.md) and the editions (api/editions/editions.json) */
final class Catalog
{
    private static ?array $data = null;
    /** @var array<string, array<string, mixed>>|null */
    private static ?array $items = null;
    /** @var array<string, array<string, mixed>>|null */
    private static ?array $editions = null;

    public static function path(): string
    {
        return __DIR__ . '/../../config/items.json';
    }

    /** @return array<string, mixed> the whole catalog as served by GET /api/items/ */
    public static function data(): array
    {
        if (self::$data === null) {
            $data = json_decode((string) file_get_contents(self::path()), true);
            if (!is_array($data) || !isset($data['items'])) throw new RuntimeException('items.json is invalid');
            self::$data = $data;
        }
        return self::$data;
    }

    /** @return array<string, array<string, mixed>> id => item */
    public static function items(): array
    {
        if (self::$items === null) {
            self::$items = [];
            foreach (self::data()['items'] as $item) self::$items[$item['id']] = $item;
        }
        return self::$items;
    }

    /** @return array<string, mixed>|null */
    public static function item(string $id): ?array
    {
        return self::items()[$id] ?? null;
    }

    /** @return string[] */
    public static function slots(): array
    {
        return self::data()['slots'] ?? [];
    }

    /** @return array<string, mixed>|null */
    public static function recipe(string $id): ?array
    {
        foreach (self::data()['recipes'] ?? [] as $recipe) {
            if ($recipe['id'] === $id) return $recipe;
        }
        return null;
    }

    /** @return string[] variant ids of an item ([] = no variants) */
    public static function variantIds(array $item): array
    {
        return array_map(fn($v) => (string) $v['id'], $item['variants'] ?? []);
    }

    /** @return array<string, mixed>|null */
    public static function variant(array $item, ?string $variantId): ?array
    {
        foreach ($item['variants'] ?? [] as $variant) {
            if ($variant['id'] === $variantId) return $variant;
        }
        return null;
    }

    /** @return list<array<string, mixed>> catalog items that belong to an edition (the claim options of its codes) */
    public static function itemsForEdition(string $editionId): array
    {
        return array_values(array_filter(self::items(), fn($item) => ($item['edition_id'] ?? null) === $editionId));
    }

    /** @return list<array<string, mixed>> starter items (every new inventory gets one) */
    public static function starters(): array
    {
        return array_values(array_filter(self::items(), fn($item) => !empty($item['starter'])));
    }

    /**
     * Smallest scan cooldown (seconds) among the tools whose scan_object skill grants this item; null = no tool grants it.
     * @return array{tools: string[], cooldown: int}|null
     */
    public static function scanTools(string $itemId): ?array
    {
        $tools = [];
        $cooldown = null;
        foreach (self::items() as $item) {
            foreach ($item['skills'] ?? [] as $skill) {
                if (($skill['type'] ?? '') === 'scan_object' && ($skill['grants'] ?? null) === $itemId) {
                    $tools[] = $item['id'];
                    $seconds = (int) ($skill['cooldown_s'] ?? 0);
                    $cooldown = $cooldown === null ? $seconds : min($cooldown, $seconds);
                }
            }
        }
        return $tools ? ['tools' => $tools, 'cooldown' => (int) $cooldown] : null;
    }

    /** @return array<string, mixed>|null the edition from editions.json */
    public static function edition(?string $id): ?array
    {
        if ($id === null || $id === '') return null;
        if (self::$editions === null) {
            self::$editions = [];
            $list = json_decode((string) file_get_contents(__DIR__ . '/../../config/editions.json'), true) ?: [];
            foreach ($list as $edition) self::$editions[$edition['id']] = $edition;
        }
        return self::$editions[$id] ?? null;
    }

    /** Localised text of a { de, en } map */
    public static function text(mixed $map, string $locale): string
    {
        if (!is_array($map)) return (string) $map;
        return (string) ($map[$locale] ?? $map['en'] ?? reset($map) ?? '');
    }

    /** Public URL of a CDN path (icons) */
    public static function cdnUrl(?string $path): ?string
    {
        if ($path === null || $path === '') return null;
        return rtrim(Config::get('CDN_URL'), '/') . '/' . ltrim($path, '/');
    }
}
