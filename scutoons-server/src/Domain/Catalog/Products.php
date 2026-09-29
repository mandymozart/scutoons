<?php
declare(strict_types=1);

namespace Scutoons\Domain\Catalog;

use RuntimeException;
use Scutoons\Infrastructure\Config\Settings;

/**
 * The shop catalog (config/products.json, version 2: `{ version, currency, checked_at, source, products:[…] }`) and
 * the products featured on the homepage (config/featured-products.json: `[{ product, variant? }]`). Types:
 * @scutoons/content `ProductCatalog` / `FeaturedProduct`. Served byte-for-byte as stored (was the viewer's
 * static/api/products.json and featured-products.json), so the files stay the single source.
 */
final class Products
{
    public function __construct(private readonly Settings $settings)
    {
    }

    public function catalogPath(): string
    {
        return $this->settings->path('PRODUCTS_PATH');
    }

    public function featuredPath(): string
    {
        return $this->settings->path('FEATURED_PRODUCTS_PATH');
    }

    /** The raw JSON of config/products.json (checked to be a catalog with a `products` list) */
    public function catalogJson(): string
    {
        return self::read($this->catalogPath(), fn(mixed $data) => is_array($data) && is_array($data['products'] ?? null));
    }

    /** The raw JSON of config/featured-products.json (checked to be a list) */
    public function featuredJson(): string
    {
        return self::read($this->featuredPath(), fn(mixed $data) => is_array($data) && array_is_list($data));
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
        return (array) json_decode($this->featuredJson(), true);
    }

    /** @param callable(mixed): bool $valid */
    private static function read(string $path, callable $valid): string
    {
        $raw = @file_get_contents($path);
        if ($raw === false || !$valid(json_decode($raw, true))) {
            throw new RuntimeException(basename($path) . ' is missing or invalid');
        }
        return $raw;
    }
}
