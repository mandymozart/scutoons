<?php
declare(strict_types=1);

namespace Scutoons\Domain\Catalog;

use RuntimeException;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Database\Database;
use stdClass;

/**
 * The seed of the catalog tables: server/config/*.json (items.json, products.json, editions.json, toons.json,
 * featured-products.json, permissions.json – the snapshot of 2026-10-04, agents/CATALOG.md). Used for
 *   - `catalog:seed`        empty tables of a local / dev database ← the files (`--replace`: overwrite);
 *   - `catalog:export-sql`  the INSERTs of the migration 2026-10-04-catalog (production gets today's data that way);
 *   - the fallback of CatalogStore before the migration ran;
 *   - the respawn / featured "default" of the admin (the rule / list in the files).
 * The files' `slots` / `stats` are code now (AvatarSlots, ReaderStats) and are ignored here.
 */
final class CatalogSeed
{
    /** updated_at / updated_by of the seed rows */
    public const SEEDED_AT = '2026-10-04 00:00:00';
    public const SEEDED_BY = 'seed';

    /** @var array<string, list<array{id: string, position: int, data: string}>>|null */
    private ?array $rows = null;

    public function __construct(private readonly Settings $settings)
    {
    }

    /**
     * Every kind's rows from the files, in file order
     * @return array<string, list<array{id: string, position: int, data: string}>>
     */
    public function rows(): array
    {
        if ($this->rows !== null) return $this->rows;
        $items = self::object($this->file('ITEMS_PATH'));
        $products = self::object($this->file('PRODUCTS_PATH'));
        $permissions = self::object($this->file('PERMISSIONS_PATH'));
        $entries = array_fill_keys(array_keys(CatalogStore::KINDS), []);
        foreach (['items', 'recipes', 'albums'] as $kind) {
            foreach ((array) ($items->$kind ?? []) as $entry) $entries[$kind][] = [self::id($entry), $entry];
        }
        foreach ((array) ($products->products ?? []) as $entry) $entries['products'][] = [self::id($entry), $entry];
        foreach (self::listOf($this->file('EDITIONS_PATH')) as $entry) $entries['editions'][] = [self::id($entry), $entry];
        foreach (self::listOf($this->file('TOONS_PATH')) as $entry) $entries['toons'][] = [self::id($entry), $entry];
        foreach (self::listOf($this->file('FEATURED_PRODUCTS_PATH')) as $entry) {
            $entries['featured'][] = [CatalogStore::featuredId((string) ($entry->product ?? ''), isset($entry->variant) ? (string) $entry->variant : null), $entry];
        }
        foreach ((array) ($permissions->permissions ?? []) as $id => $description) $entries['permissions'][] = [(string) $id, $description];
        foreach ((array) ($permissions->types ?? []) as $id => $type) $entries['code-types'][] = [(string) $id, $type];
        if (isset($permissions->default)) $entries['code-types'][] = [CatalogStore::DEFAULT_CODE_TYPE, $permissions->default];
        $entries['meta'] = [
            ['items', (object) ['version' => $items->version ?? 1]],
            ['products', self::without($products, 'products')],
            ['permissions', (object) ['version' => $permissions->version ?? 1]],
        ];
        $this->rows = [];
        foreach ($entries as $kind => $list) {
            $this->rows[$kind] = array_map(fn(array $e, int $i) => ['id' => $e[0], 'position' => $i, 'data' => CatalogStore::encode($e[1])], $list, array_keys($list));
        }
        return $this->rows;
    }

    /** One entry of the files (decoded as arrays), null = not in the seed */
    public function entry(string $kind, string $id): mixed
    {
        foreach ($this->rows()[$kind] ?? [] as $row) {
            if ($row['id'] === $id) return json_decode($row['data'], true);
        }
        return null;
    }

    /**
     * Fills the tables from the files: only empty ones, or – $replace – every one (its rows replaced)
     * @return array<string, int|null> kind → rows written (null = left as it was)
     */
    public function seed(Database $db, bool $replace = false): array
    {
        $counts = [];
        $db->transaction(function () use ($db, $replace, &$counts) {
            foreach ($this->rows() as $kind => $rows) {
                $table = CatalogStore::table($kind);
                if (!$replace && (int) $db->value("SELECT COUNT(*) FROM $table") > 0) {
                    $counts[$kind] = null;
                    continue;
                }
                $db->run("DELETE FROM $table");
                foreach ($rows as $row) {
                    $columns = CatalogStore::columns($kind, json_decode($row['data'], true));
                    $names = implode('', array_map(fn(string $c) => ", $c", array_keys($columns)));
                    $db->run("INSERT INTO $table (id, position, data, updated_at, updated_by$names) VALUES (?, ?, ?, ?, ?" . str_repeat(', ?', count($columns)) . ')',
                        [$row['id'], $row['position'], $row['data'], self::SEEDED_AT, self::SEEDED_BY, ...array_values($columns)]);
                }
                $counts[$kind] = count($rows);
            }
        });
        return $counts;
    }

    /**
     * The seed as one INSERT per table for a driver, each only while that table is EMPTY – running the migration again
     * never brings back an entry an admin deleted, nor mixes rows into a list (one row per line, the first line names
     * the columns)
     * @return list<string>
     */
    public function inserts(string $driver): array
    {
        $out = [];
        $q = fn(string $name) => $driver === 'mysql' ? "`$name`" : $name;
        foreach ($this->rows() as $kind => $rows) {
            if ($rows === []) continue;
            $table = $q(CatalogStore::table($kind));
            $lines = [];
            $header = '';
            foreach ($rows as $i => $row) {
                $columns = CatalogStore::columns($kind, json_decode($row['data'], true));
                $names = ['id', 'position', 'data', 'updated_at', 'updated_by', ...array_keys($columns)];
                $values = [$row['id'], $row['position'], $row['data'], self::SEEDED_AT, self::SEEDED_BY, ...array_values($columns)];
                $lines[] = ($i === 0 ? '  SELECT ' : '  UNION ALL SELECT ') . implode(', ', array_map(
                    fn($v, $n) => self::literal($v, $driver) . ($i === 0 ? ' AS ' . $q($n) : ''), $values, $names));
                if ($i === 0) $header = "INSERT INTO $table (" . implode(', ', array_map($q, $names)) . ') SELECT * FROM (';
            }
            $out[] = $header . "\n" . implode("\n", $lines) . "\n) AS seed WHERE NOT EXISTS (SELECT 1 FROM $table);";
        }
        return $out;
    }

    /** A SQL literal (MariaDB: backslashes escaped too – the server does not run with NO_BACKSLASH_ESCAPES) */
    public static function literal(mixed $value, string $driver): string
    {
        if ($value === null) return 'NULL';
        if (is_int($value)) return (string) $value;
        $text = (string) $value;
        if ($driver === 'mysql') $text = str_replace('\\', '\\\\', $text);
        return "'" . str_replace("'", "''", $text) . "'";
    }

    private function file(string $setting): mixed
    {
        $path = $this->settings->path($setting);
        $raw = $path !== '' ? @file_get_contents($path) : false;
        if ($raw === false) throw new RuntimeException("Catalog seed file missing: $setting ($path)");
        $data = json_decode($raw);
        if ($data === null) throw new RuntimeException("Catalog seed file invalid: $path");
        return $data;
    }

    private static function object(mixed $data): stdClass
    {
        return $data instanceof stdClass ? $data : new stdClass();
    }

    /** @return list<stdClass> */
    private static function listOf(mixed $data): array
    {
        return is_array($data) ? array_values(array_filter($data, fn($e) => $e instanceof stdClass)) : [];
    }

    private static function id(mixed $entry): string
    {
        return $entry instanceof stdClass && is_scalar($entry->id ?? null) ? (string) $entry->id : '';
    }

    private static function without(stdClass $data, string $key): stdClass
    {
        $copy = clone $data;
        unset($copy->$key);
        return $copy;
    }
}
