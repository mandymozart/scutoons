<?php
declare(strict_types=1);

namespace Scutoons\Domain\Catalog;

use PDOException;
use Scutoons\Domain\ApiError;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Time;

/**
 * The editions featured on the landing page (owner 2026-10-01: chosen in the admin app, page "Startseite"): an ordered list
 * `[{ product, variant? }]` of the shop catalog – the table catalog_featured (agents/CATALOG.md; until 2026-10-04 the
 * runtime setting `featured_products` over config/featured-products.json – the migration 2026-10-04-catalog moved it
 * into the table). The "default" is the seed's list (config/featured-products.json).
 *   GET  /api/products/featured/  the list (public)
 *   GET  /api/admin/featured      { items, default, stored, updated_at, updated_by }
 *   PUT  /api/admin/featured      { items: [...] | null } (null = back to the default) – one catalog_history row
 */
final class FeaturedProducts
{
    /** the retired setting (migration 2026-10-04-catalog) */
    public const SETTING = 'featured_products';
    public const MAX_ITEMS = 12;

    public function __construct(private readonly Database $db, private readonly Products $products, private readonly CatalogSeed $seed, private readonly CatalogAdmin $admin)
    {
    }

    /** The public answer */
    public function json(): string
    {
        return $this->products->featuredJson();
    }

    /**
     * The admin view: the list, the default, whether it differs from the default and its last change
     * @return array{items: list<array<string, mixed>>, default: list<array<string, mixed>>, stored: bool, updated_at: ?string, updated_by: ?string, max: int}
     */
    public function state(): array
    {
        $items = $this->products->featured();
        $default = $this->defaults();
        try {
            $last = $this->db->one("SELECT changed_at, changed_by FROM catalog_history WHERE kind = 'featured' ORDER BY id DESC LIMIT 1");
        } catch (PDOException) {
            $last = null; // before the migration
        }
        return [
            'items' => $items,
            'default' => $default,
            'stored' => $items !== $default,
            'updated_at' => Time::iso($last['changed_at'] ?? null),
            'updated_by' => isset($last['changed_by']) ? (string) $last['changed_by'] : null,
            'max' => self::MAX_ITEMS,
        ];
    }

    /**
     * Replaces the list (validated against the catalog: known product, known variant, no duplicates, at most MAX_ITEMS),
     * or – null – goes back to the default. Returns whether anything changed.
     * @param mixed $items list of { product, variant? } or null
     */
    public function update(mixed $items, string $by): bool
    {
        $clean = $items === null ? $this->defaults() : $this->validate($items);
        return $this->admin->replace('featured', array_map(fn(array $i) => (object) $i, $clean), $by)['changed'];
    }

    /** @return list<array<string, mixed>> the seed's list */
    private function defaults(): array
    {
        return array_values(array_filter(array_map(fn(array $r) => json_decode($r['data'], true), $this->seed->rows()['featured']), 'is_array'));
    }

    /**
     * @return list<array{product: string, variant?: string}>
     */
    private function validate(mixed $items): array
    {
        if (!is_array($items) || !array_is_list($items)) {
            throw new ApiError(400, 'invalid_featured', 'items must be a list of { product, variant? } (or null for the default).');
        }
        if (count($items) > self::MAX_ITEMS) {
            throw new ApiError(400, 'invalid_featured', 'At most ' . self::MAX_ITEMS . ' featured products.', ['max' => self::MAX_ITEMS]);
        }
        $catalog = [];
        foreach ((array) ($this->products->catalog()['products'] ?? []) as $product) {
            if (is_array($product) && is_string($product['id'] ?? null)) {
                $catalog[$product['id']] = array_values(array_filter(array_map(fn($v) => is_array($v) ? ($v['id'] ?? null) : null, (array) ($product['variants'] ?? [])), 'is_string'));
            }
        }
        $clean = [];
        $seen = [];
        foreach ($items as $index => $item) {
            $product = is_array($item) ? ($item['product'] ?? null) : null;
            $variant = is_array($item) ? ($item['variant'] ?? null) : null;
            if (!is_string($product) || ($variant !== null && !is_string($variant))) {
                throw new ApiError(400, 'invalid_featured', 'Each item is { product, variant? }.', ['index' => $index]);
            }
            if (!isset($catalog[$product])) {
                throw new ApiError(400, 'unknown_product', "Unknown product $product.", ['index' => $index, 'product' => $product, 'products' => array_keys($catalog)]);
            }
            if ($variant !== null && $variant !== '' && !in_array($variant, $catalog[$product], true)) {
                throw new ApiError(400, 'unknown_variant', "Unknown variant $variant of $product.", ['index' => $index, 'product' => $product, 'variant' => $variant, 'variants' => $catalog[$product]]);
            }
            $id = $product . '/' . ($variant ?? '');
            if (isset($seen[$id])) {
                throw new ApiError(400, 'duplicate_featured', "$product" . ($variant ? " ($variant)" : '') . ' is in the list twice.', ['index' => $index, 'product' => $product]);
            }
            $seen[$id] = true;
            $clean[] = $variant !== null && $variant !== '' ? ['product' => $product, 'variant' => $variant] : ['product' => $product];
        }
        return $clean;
    }
}
