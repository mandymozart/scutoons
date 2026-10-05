<?php
declare(strict_types=1);

namespace Scutoons\Domain\Catalog;

use Scutoons\Domain\ApiError;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Database\Database;
use stdClass;

/**
 * The catalog tables (agents/CATALOG.md, migration 2026-10-04-catalog): one row per entry, its definition as JSON in
 * the shape of the old config files (`data`), the order (`position`) and the columns lookups need (toon, item_id,
 * edition_id …, kept in step with `data` on every write). Every reader of the catalog goes through here (Catalog,
 * Editions, Products, FeaturedProducts, Toons, Permissions, AssetUsages); writes with validation + history are
 * CatalogAdmin's.
 */
final class CatalogStore
{
    /**
     * kind → table, the columns derived from `data` (column → data field), the label of its asset usages / history
     * (`<label>:<id>`), whether `data.id` is the entry id
     */
    public const KINDS = [
        'items' => ['table' => 'catalog_items', 'columns' => ['toon' => 'toon', 'edition_id' => 'edition_id'], 'label' => 'item', 'data_id' => true],
        'recipes' => ['table' => 'catalog_recipes', 'columns' => [], 'label' => 'recipe', 'data_id' => true],
        'albums' => ['table' => 'catalog_albums', 'columns' => ['toon' => 'toon'], 'label' => 'album', 'data_id' => true],
        'products' => ['table' => 'catalog_products', 'columns' => ['toon' => 'toon', 'item_id' => 'item_id', 'edition_id' => 'edition_id'], 'label' => 'product', 'data_id' => true],
        'editions' => ['table' => 'catalog_editions', 'columns' => ['toon' => 'toon'], 'label' => 'edition', 'data_id' => true],
        'featured' => ['table' => 'catalog_featured', 'columns' => ['product_id' => 'product', 'variant_id' => 'variant'], 'label' => 'featured', 'data_id' => false],
        'permissions' => ['table' => 'catalog_permissions', 'columns' => [], 'label' => 'permission', 'data_id' => false],
        'code-types' => ['table' => 'catalog_code_types', 'columns' => [], 'label' => 'code-type', 'data_id' => false],
        'toons' => ['table' => 'toons', 'columns' => [], 'label' => 'toon', 'data_id' => true],
        'meta' => ['table' => 'catalog_meta', 'columns' => [], 'label' => 'meta', 'data_id' => false],
    ];
    /** `data` as stored: like the files (slashes / unicode unescaped, 1.0 stays 1.0) */
    public const JSON_FLAGS = JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_PRESERVE_ZERO_FRACTION;
    /** the code type entry that holds permissions.json `default` (types not listed) */
    public const DEFAULT_CODE_TYPE = 'default';
    /** the meta entries: items.json / products.json / permissions.json without their lists */
    public const META_IDS = ['items', 'products', 'permissions'];
    /**
     * The admin's accepted catalog warnings in an entry's data (@scutoons/content accepted.ts `catalogWarningPlace`):
     * versioned with the entry (history, undo), never served to readers – list() / map() / get() / reader() drop it.
     */
    public const ACCEPTED_WARNINGS = 'accepted_warnings';

    /** @var array<string, list<array{id: string, position: int, data: string, updated_at: ?string, updated_by: ?string}>> */
    private array $rows = [];
    private int $generation = 0;

    public function __construct(private readonly Database $db, private readonly Clock $clock)
    {
    }

    public static function kind(string $kind): string
    {
        if (!isset(self::KINDS[$kind])) throw new ApiError(404, 'unknown_kind', "Unknown catalog kind $kind.", ['kinds' => array_keys(self::KINDS)]);
        return $kind;
    }

    public static function table(string $kind): string
    {
        return self::KINDS[self::kind($kind)]['table'];
    }

    /** The id of a featured entry: `<product>` or `<product>:<variant>` */
    public static function featuredId(string $product, ?string $variant): string
    {
        return $variant !== null && $variant !== '' ? "$product:$variant" : $product;
    }

    /**
     * The derived columns of an entry (column → value)
     * @return array<string, string|null>
     */
    public static function columns(string $kind, mixed $data): array
    {
        $out = [];
        foreach (self::KINDS[$kind]['columns'] as $column => $field) {
            $value = is_array($data) ? ($data[$field] ?? null) : null;
            $out[$column] = is_string($value) && $value !== '' ? $value : null;
        }
        return $out;
    }

    public static function encode(mixed $data): string
    {
        $json = json_encode($data, self::JSON_FLAGS);
        if ($json === false) throw new ApiError(422, 'invalid_catalog', 'data could not be encoded.');
        return $json;
    }

    /**
     * The rows of a kind in their order
     * @return list<array{id: string, position: int, data: string, updated_at: ?string, updated_by: ?string}>
     */
    public function rows(string $kind): array
    {
        $kind = self::kind($kind);
        if (isset($this->rows[$kind])) return $this->rows[$kind];
        $rows = $this->db->all('SELECT id, position, data, updated_at, updated_by FROM ' . self::KINDS[$kind]['table'] . ' ORDER BY position, id');
        return $this->rows[$kind] = array_map(fn(array $r) => [
            'id' => (string) $r['id'],
            'position' => (int) $r['position'],
            'data' => (string) $r['data'],
            'updated_at' => $r['updated_at'] !== null ? (string) $r['updated_at'] : null,
            'updated_by' => $r['updated_by'] !== null ? (string) $r['updated_by'] : null,
        ], $rows);
    }

    /**
     * An entry's data as readers get it: decoded (objects as arrays, or stdClass with $assoc false) without the admin's
     * accepted warnings
     */
    public static function reader(string $json, bool $assoc = true): mixed
    {
        $data = json_decode($json, $assoc);
        if (is_array($data) && !array_is_list($data)) unset($data[self::ACCEPTED_WARNINGS]);
        elseif ($data instanceof stdClass) unset($data->{self::ACCEPTED_WARNINGS});
        return $data;
    }

    /** @return list<mixed> the decoded entries in their order (objects as arrays) */
    public function list(string $kind): array
    {
        return array_map(fn(array $r) => self::reader($r['data']), $this->rows($kind));
    }

    /** @return array<string, mixed> id → decoded entry, in order */
    public function map(string $kind): array
    {
        $out = [];
        foreach ($this->rows($kind) as $r) $out[$r['id']] = self::reader($r['data']);
        return $out;
    }

    /** @return array{id: string, position: int, data: string, updated_at: ?string, updated_by: ?string}|null */
    public function row(string $kind, string $id): ?array
    {
        foreach ($this->rows($kind) as $r) {
            if ($r['id'] === $id) return $r;
        }
        return null;
    }

    public function get(string $kind, string $id): mixed
    {
        $row = $this->row($kind, $id);
        return $row === null ? null : self::reader($row['data']);
    }

    /** Forgets what was read (after a write; another request reads again anyway) */
    public function forget(): void
    {
        $this->rows = [];
        $this->generation++;
    }

    /** Changes with every write: readers that keep decoded data (Catalog) read again when it moved */
    public function generation(): int
    {
        return $this->generation;
    }

    // ── Writes (no validation, no history – CatalogAdmin; inside the caller's transaction) ─────────────────────

    /**
     * Inserts or updates an entry; a new one goes to the end unless $position is given. $data decoded with objects
     * (stdClass) keeps `{}` an object.
     */
    public function save(string $kind, string $id, mixed $data, string $by, ?int $position = null): void
    {
        $table = self::table($kind);
        $json = self::encode($data);
        $columns = self::columns($kind, json_decode($json, true));
        $exists = $this->db->value("SELECT 1 FROM $table WHERE id = ?", [$id]) !== null;
        $now = $this->clock->utc();
        if ($exists) {
            $set = implode('', array_map(fn(string $c) => ", $c = ?", array_keys($columns)));
            $params = [$json, $now, substr($by, 0, 255), ...array_values($columns)];
            if ($position !== null) {
                $set .= ', position = ?';
                $params[] = $position;
            }
            $this->db->run("UPDATE $table SET data = ?, updated_at = ?, updated_by = ?$set WHERE id = ?", [...$params, $id]);
        } else {
            $position ??= (int) $this->db->value("SELECT COALESCE(MAX(position), -1) FROM $table") + 1;
            $names = implode('', array_map(fn(string $c) => ", $c", array_keys($columns)));
            $marks = str_repeat(', ?', count($columns));
            $this->db->run("INSERT INTO $table (id, position, data, updated_at, updated_by$names) VALUES (?, ?, ?, ?, ?$marks)",
                [$id, $position, $json, $now, substr($by, 0, 255), ...array_values($columns)]);
        }
        $this->forget();
    }

    public function remove(string $kind, string $id): void
    {
        $this->db->run('DELETE FROM ' . self::table($kind) . ' WHERE id = ?', [$id]);
        $this->forget();
    }

    /**
     * Sets the order: the given ids first (unknown ones skipped), the rest after them in their current order
     * @param list<string> $ids
     * @return list<string> the resulting order
     */
    public function reorder(string $kind, array $ids): array
    {
        $current = array_column($this->rows($kind), 'id');
        $order = array_values(array_unique([...array_values(array_filter($ids, fn($id) => in_array($id, $current, true))), ...$current]));
        $table = self::table($kind);
        foreach ($order as $position => $id) $this->db->run("UPDATE $table SET position = ? WHERE id = ?", [$position, $id]);
        $this->forget();
        return $order;
    }
}
