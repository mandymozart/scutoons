<?php
declare(strict_types=1);

namespace Scutoons\Domain\Embassy;

use PDOException;
use Scutoons\Domain\ApiError;
use Scutoons\Infrastructure\Database\Database;

/**
 * The public "Where to buy" list (GET /api/stockists/, scutoons.com/stores; agents/EMBASSY.md "Stores"): the Embassy's
 * stores (Embassy::STOCKIST_TYPES) that are `listed` and carry at least one product of the shop catalog, with their
 * PUBLIC fields only – never people, emails, phones, notes, tags, tier, source, leads or activities.
 *
 * Small / independent vendors come first (owner 2026-10-02: "I want to encourage small vendors at first"), then by
 * country, city and name. Product ids that are no longer in the catalog are dropped; a store left with none is not listed.
 */
final class Stockists
{
    public function __construct(private readonly Database $db, private readonly Embassy $embassy)
    {
    }

    /**
     * @param array<string, mixed> $filters product (a product id), country (ISO code)
     * @return array{stores: list<array<string, mixed>>, products: array<string, int>, countries: list<string>}
     *   `stores` filtered; `products` (product id → number of listed stores carrying it) and `countries` over all listed
     *   stores, so a page can offer the filters and a product page can ask "is it in a shop?" with the same answer
     */
    public function list(array $filters = []): array
    {
        $product = trim((string) ($filters['product'] ?? ''));
        if ($product !== '' && !preg_match('/^[A-Za-z0-9_-]{1,64}$/', $product)) {
            throw new ApiError(400, 'invalid_product', 'product must be a product id.', ['field' => 'product']);
        }
        $country = strtoupper(trim((string) ($filters['country'] ?? '')));
        if ($country !== '' && !preg_match('/^[A-Z]{2}$/', $country)) {
            throw new ApiError(400, 'invalid_country', 'Countries are ISO codes like AT, DE, NL.', ['field' => 'country']);
        }

        $all = $this->listedStores();
        $products = [];
        $countries = [];
        foreach ($all as $store) {
            foreach ($store['carries'] as $id) $products[$id] = ($products[$id] ?? 0) + 1;
            if ($store['country'] !== '') $countries[$store['country']] = true;
        }
        ksort($products);
        $countries = array_keys($countries);
        sort($countries);

        $stores = array_values(array_filter($all, fn(array $s) => ($product === '' || in_array($product, $s['carries'], true))
            && ($country === '' || $s['country'] === $country)));
        return ['stores' => $stores, 'products' => $products, 'countries' => $countries];
    }

    /** @return list<array<string, mixed>> every listed store with its public fields, in the page's order */
    private function listedStores(): array
    {
        $types = Embassy::STOCKIST_TYPES;
        try {
            $rows = $this->db->all(
                'SELECT id, name, type, city, country, address, website, opening, independent, carries, name_key FROM embassy_orgs
                 WHERE listed = 1 AND type IN (' . implode(', ', array_fill(0, count($types), '?')) . ')',
                $types,
            );
        } catch (PDOException $error) {
            // before the migrations 2026-10-01-embassy / 2026-10-02-embassy-stores ran: nobody is listed yet
            if (!self::missingSchema($error)) throw $error;
            return [];
        }
        $known = array_flip($this->embassy->productIds());
        $stores = [];
        foreach ($rows as $r) {
            $carries = json_decode((string) $r['carries'], true);
            $carries = is_array($carries) ? array_values(array_filter($carries, fn($id) => is_string($id) && isset($known[$id]))) : [];
            if (!$carries) continue;
            $stores[] = [
                'id' => (string) $r['id'],
                'name' => (string) $r['name'],
                'type' => (string) $r['type'],
                'independent' => (int) $r['independent'] === 1,
                'city' => (string) $r['city'],
                'country' => (string) $r['country'],
                'address' => (string) $r['address'],
                'website' => self::publicUrl((string) $r['website']),
                'opening' => (string) $r['opening'],
                'carries' => $carries,
                '_sort' => [(int) $r['independent'] === 1 ? 0 : 1, (string) $r['country'], Embassy::nameKey((string) $r['city']), (string) $r['name_key']],
            ];
        }
        usort($stores, fn(array $a, array $b) => $a['_sort'] <=> $b['_sort']);
        return array_map(function (array $s): array {
            unset($s['_sort']);
            return $s;
        }, $stores);
    }

    /** Only http(s) links leave the Embassy (a `mailto:` or a note in the website field stays internal) */
    private static function publicUrl(string $url): string
    {
        return preg_match('#^https?://[^\s]+$#i', $url) ? $url : '';
    }

    /** MariaDB 42S02 (table) / 42S22 (column) */
    private static function missingSchema(PDOException $error): bool
    {
        return in_array($error->getCode(), ['42S02', '42S22'], true)
            || preg_match('/Base table or view not found|Unknown column/i', $error->getMessage()) === 1;
    }
}
