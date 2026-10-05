<?php
declare(strict_types=1);

namespace Scutoons\Domain\Catalog;

use stdClass;

/**
 * The shop catalog (table catalog_products + the meta entry `products`: `{ version, currency, checked_at, source,
 * products:[…] }`, the shape of the old config/products.json) and the products featured on the homepage (table
 * catalog_featured: `[{ product, variant? }]`). Types: @scutoons/content `ProductCatalog` / `FeaturedProduct`. Served
 * in the files' shape (objects stay objects, the stored key order) – agents/CATALOG.md.
 */
final class Products
{
    private const JSON_FLAGS = JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_PRESERVE_ZERO_FRACTION;

    public function __construct(private readonly CatalogStore $store)
    {
    }

    /** The JSON of the shop catalog (GET /api/products/) */
    public function catalogJson(): string
    {
        $meta = $this->store->row('meta', 'products');
        $catalog = $meta !== null ? CatalogStore::reader($meta['data'], false) : null;
        if (!$catalog instanceof stdClass) $catalog = (object) ['version' => 2];
        $catalog->products = array_map(fn(array $r) => CatalogStore::reader($r['data'], false), $this->store->rows('products'));
        return (string) json_encode($catalog, self::JSON_FLAGS);
    }

    /** The JSON of the featured list (GET /api/products/featured/) */
    public function featuredJson(): string
    {
        return (string) json_encode(array_map(fn(array $r) => CatalogStore::reader($r['data'], false), $this->store->rows('featured')), self::JSON_FLAGS);
    }

    /** @return array<string, mixed> the decoded catalog */
    public function catalog(): array
    {
        return (array) json_decode($this->catalogJson(), true);
    }

    /** @return list<array<string, mixed>> the decoded featured list */
    public function featured(): array
    {
        /** @var list<array<string, mixed>> */
        return array_values(array_filter($this->store->list('featured'), 'is_array'));
    }

    /** @return array<string, mixed>|null one product */
    public function product(string $id): ?array
    {
        $product = $this->store->get('products', $id);
        return is_array($product) ? $product : null;
    }
}
