<?php
declare(strict_types=1);

namespace Scutoons\Domain\Catalog;

use PDOException;
use Scutoons\Domain\ApiError;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Time;

/**
 * The editions featured on the landing page (owner 2026-10-01: chosen in the admin app, page "Startseite"): an ordered list
 * `[{ product, variant? }]` of the shop catalog (config/products.json). Stored as the runtime setting `featured_products`
 * (table `settings`, like the rate limits: JSON value, updated_at, updated_by = `admin:<email>`); without a row – or
 * before the `settings` migration – the default is config/featured-products.json (dago_hoodie beige, dago_heavy_t white).
 *   GET  /api/products/featured/  the effective list (public)
 *   GET  /api/admin/featured      { items, default, stored, updated_at, updated_by }
 *   PUT  /api/admin/featured      { items: [...] | null } (null = back to the default)
 */
final class FeaturedProducts
{
    public const SETTING = 'featured_products';
    public const MAX_ITEMS = 12;
    private const JSON_FLAGS = JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE;

    public function __construct(private readonly Database $db, private readonly Products $products, private readonly Clock $clock)
    {
    }

    /** The public answer: the stored list as JSON, else the default file byte-for-byte */
    public function json(): string
    {
        $stored = $this->stored();
        return $stored === null ? $this->products->featuredJson() : (string) json_encode($stored['items'], self::JSON_FLAGS);
    }

    /**
     * The admin view: the effective list, the default, whether a list is stored and its last change
     * @return array{items: list<array<string, mixed>>, default: list<array<string, mixed>>, stored: bool, updated_at: ?string, updated_by: ?string, max: int}
     */
    public function state(): array
    {
        $stored = $this->stored();
        $default = $this->products->featured();
        return [
            'items' => $stored['items'] ?? $default,
            'default' => $default,
            'stored' => $stored !== null,
            'updated_at' => $stored !== null ? Time::iso($stored['updated_at']) : null,
            'updated_by' => $stored['updated_by'] ?? null,
            'max' => self::MAX_ITEMS,
        ];
    }

    /**
     * Replaces the list (validated against the catalog: known product, known variant, no duplicates, at most MAX_ITEMS),
     * or – null – goes back to the default (row removed). Returns whether anything changed.
     * @param mixed $items list of { product, variant? } or null
     */
    public function update(mixed $items, string $by): bool
    {
        $stored = $this->stored();
        if ($items === null) {
            if ($stored === null) return false;
            $this->db->run('DELETE FROM settings WHERE `key` = ?', [self::SETTING]);
            return true;
        }
        $clean = $this->validate($items);
        if ($stored !== null && $stored['items'] === $clean) return false;
        $params = [(string) json_encode($clean, self::JSON_FLAGS), $this->clock->utc(), substr($by, 0, 255), self::SETTING];
        if ($stored !== null) $this->db->run('UPDATE settings SET `value` = ?, updated_at = ?, updated_by = ? WHERE `key` = ?', $params);
        else $this->db->run('INSERT INTO settings (`value`, updated_at, updated_by, `key`) VALUES (?, ?, ?, ?)', $params);
        return true;
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

    /** @return array{items: list<array<string, mixed>>, updated_at: string, updated_by: ?string}|null */
    private function stored(): ?array
    {
        try {
            $row = $this->db->one('SELECT `value`, updated_at, updated_by FROM settings WHERE `key` = ?', [self::SETTING]);
        } catch (PDOException) {
            return null; // no settings table yet (migration pending): the default
        }
        if ($row === null) return null;
        $items = json_decode((string) $row['value'], true);
        if (!is_array($items) || !array_is_list($items)) return null;
        /** @var list<array<string, mixed>> $items */
        return [
            'items' => $items,
            'updated_at' => (string) $row['updated_at'],
            'updated_by' => $row['updated_by'] !== null ? (string) $row['updated_by'] : null,
        ];
    }
}
