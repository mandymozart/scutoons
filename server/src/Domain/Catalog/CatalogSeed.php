<?php
declare(strict_types=1);

namespace Scutoons\Domain\Catalog;

use RuntimeException;
use Scutoons\Infrastructure\Database\Database;
use stdClass;

/**
 * Catalog files in the shape of the old server/config/*.json (items.json, products.json, editions.json, toons.json,
 * featured-products.json, permissions.json) → the catalog tables. Only for tests: the fixture
 * server/tests/fixtures/catalog (FIXTURE) is the 2026-10-04 snapshot – the same rows the migration 2026-10-04-catalog
 * inserts into a fresh database. Production is the catalog (agents/CATALOG.md); a local database gets it from
 * `db:prod-seed` (agents/CUTOVER.md 10.30). The files' `slots` / `stats` are code (AvatarSlots, ReaderStats).
 */
final class CatalogSeed
{
    /** the committed test fixture, relative to the server folder */
    public const FIXTURE = 'tests/fixtures/catalog';
    /** updated_at / updated_by of the seed rows */
    public const SEEDED_AT = '2026-10-04 00:00:00';
    public const SEEDED_BY = 'seed';

    /** @var array<string, list<array{id: string, position: int, data: string}>>|null */
    private ?array $rows = null;

    /** @param string $dir a folder with the catalog files (e.g. the server root + FIXTURE) */
    public function __construct(private readonly string $dir)
    {
    }

    /**
     * Every kind's rows from the files, in file order
     * @return array<string, list<array{id: string, position: int, data: string}>>
     */
    public function rows(): array
    {
        if ($this->rows !== null) return $this->rows;
        $items = self::object($this->file('items.json'));
        $products = self::object($this->file('products.json'));
        $permissions = self::object($this->file('permissions.json'));
        $entries = array_fill_keys(array_keys(CatalogStore::KINDS), []);
        foreach (['items', 'recipes', 'albums'] as $kind) {
            foreach ((array) ($items->$kind ?? []) as $entry) $entries[$kind][] = [self::id($entry), $entry];
        }
        // what homes look like (research/CAVE.md) – the same two rows as the migration 2026-10-14-homes
        foreach ((array) ($items->home_scenes ?? []) as $entry) $entries['home-scenes'][] = [self::id($entry), $entry];
        foreach ((array) ($items->airlift_carriers ?? []) as $entry) $entries['airlift-carriers'][] = [self::id($entry), $entry];
        foreach ((array) ($products->products ?? []) as $entry) $entries['products'][] = [self::id($entry), $entry];
        foreach (self::listOf($this->file('editions.json')) as $entry) $entries['editions'][] = [self::id($entry), $entry];
        foreach (self::listOf($this->file('toons.json')) as $entry) $entries['toons'][] = [self::id($entry), $entry];
        foreach (self::listOf($this->file('featured-products.json')) as $entry) {
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

    private function file(string $name): mixed
    {
        $path = "{$this->dir}/$name";
        $raw = @file_get_contents($path);
        if ($raw === false) throw new RuntimeException("Catalog file missing: $path");
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
