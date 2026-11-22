<?php
declare(strict_types=1);

namespace Scutoons\Domain\Catalog;

use Scutoons\Domain\Content\EntityKey;
use Scutoons\Domain\Modules\ModuleSource;

/**
 * The order of a bulk publish: what a catalog document names from the catalog goes live before it – modules →
 * instruments → presets (parents first) → rigs → device / rig presets, editions / albums → items / products → recipes –
 * so each entry's check sees what it depends on in the table. Entries that name each other (an item and its new
 * product) form one unit, published together; every other unit stands alone, so one refusal blocks only what depends on
 * it. The content documents (chapters, the map, shows …) are one unit at the end: a route and the new place it runs
 * through never go live half.
 */
final class PublishOrder
{
    /** the catalog document kinds in the order they are published when nothing else decides */
    private const RANK = ['module', 'instrument', 'preset', 'rig', 'device-preset', 'rig-preset', 'edition', 'album', 'item', 'product', 'recipe', 'home-scene', 'airlift-carrier'];

    /**
     * The catalog document keys a catalog document names (the table must have them before it can go live)
     * @return list<string>
     */
    public static function refs(EntityKey $key, mixed $data): array
    {
        if (!is_array($data)) return [];
        $refs = [];
        $add = function (string $kind, mixed $id) use (&$refs): void {
            if (is_string($id) && $id !== '' && EntityKey::isValid("$kind:$id")) $refs["$kind:$id"] = true;
        };
        switch ($key->kind) {
            case 'instrument':
                foreach (self::moduleKinds($data) as $id) $add('module', $id);
                break;
            case 'preset':
                $add('instrument', $data['instrument'] ?? null);
                $add('preset', $data['parent'] ?? null);
                break;
            case 'rig':
                foreach (is_array($data['track'] ?? null) ? $data['track'] : [] as $track) {
                    if (is_array($track)) $add('instrument', $track['instrument'] ?? null);
                }
                foreach (CatalogValidator::rigPresets($data) as $id) $add('preset', $id);
                $add('module', is_array($data['policy'] ?? null) ? ($data['policy']['module'] ?? null) : null);
                foreach (self::moduleKinds($data) as $id) $add('module', $id);
                break;
            case 'rig-preset':
                $add('rig', $data['rig'] ?? null);
                $add('rig-preset', $data['origin'] ?? null);
                break;
            case 'device-preset':
                $add('device-preset', $data['origin'] ?? null);
                break;
            case 'item':
                $add('edition', $data['edition_id'] ?? null);
                $add('product', $data['product'] ?? null);
                $add('album', $data['album'] ?? null);
                $add('instrument', $data['instrument'] ?? null);
                foreach (CatalogValidator::scanGrants($data) as $id) $add('item', $id);
                break;
            case 'product':
                $add('item', $data['item_id'] ?? null);
                $add('edition', $data['edition_id'] ?? null);
                break;
            case 'recipe':
                foreach (CatalogValidator::recipeItems($data) as $id) $add('item', $id);
                break;
        }
        unset($refs[(string) $key]);
        return array_keys($refs);
    }

    /**
     * The units of a batch in publish order: groups of keys that go live together, each after the units it names
     * @param list<string> $keys
     * @param array<string, list<string>> $refs key → the keys it names (refs())
     * @return list<list<string>>
     */
    public static function units(array $keys, array $refs): array
    {
        $catalog = [];
        $content = [];
        foreach ($keys as $key) {
            $parsed = EntityKey::parse($key);
            if ($parsed->isCatalog()) $catalog[] = $key;
            else $content[] = $key;
        }
        usort($catalog, fn(string $a, string $b) => self::rank($a) <=> self::rank($b) ?: strnatcmp($a, $b));
        $in = array_flip($catalog);
        // Tarjan's strongly connected components: each comes out after every component it names
        $index = [];
        $low = [];
        $stack = [];
        $onStack = [];
        $units = [];
        $next = 0;
        $visit = function (string $v) use (&$visit, &$index, &$low, &$stack, &$onStack, &$units, &$next, $refs, $in): void {
            $index[$v] = $low[$v] = $next++;
            $stack[] = $v;
            $onStack[$v] = true;
            foreach ($refs[$v] ?? [] as $w) {
                if (!isset($in[$w])) continue;
                if (!isset($index[$w])) {
                    $visit($w);
                    $low[$v] = min($low[$v], $low[$w]);
                } elseif (isset($onStack[$w])) {
                    $low[$v] = min($low[$v], $index[$w]);
                }
            }
            if ($low[$v] !== $index[$v]) return;
            $unit = [];
            while (($w = array_pop($stack)) !== null) {
                unset($onStack[$w]);
                $unit[] = $w;
                if ($w === $v) break;
            }
            usort($unit, fn(string $a, string $b) => self::rank($a) <=> self::rank($b) ?: strnatcmp($a, $b));
            $units[] = $unit;
        };
        foreach ($catalog as $key) {
            if (!isset($index[$key])) $visit($key);
        }
        if ($content !== []) $units[] = $content;
        return $units;
    }

    /**
     * Why a module document cannot go live as it is: no build result, a build result the platform no longer reads, or
     * sources changed since the last compile (null: it may)
     * @param array<string, mixed>|null $data the version to publish
     * @param array<string, mixed>|null $compiled the newest version a compile wrote (null: none)
     */
    public static function compileProblem(?array $data, ?array $compiled): ?string
    {
        if (!is_array($data) || CatalogDocs::isTombstone($data)) return null;
        if (!array_key_exists('descriptor', $data) || !array_key_exists('wasm', $data)) return 'not compiled yet – compile it first';
        if (ModuleSource::resultProblems($data) !== []) return 'its build is out of date – compile it first';
        if ($compiled !== null && self::sameFiles($data['files'] ?? null, $compiled['files'] ?? null) === false) return 'its sources changed since the last compile – compile it first';
        return null;
    }

    /** how a dependency is named in a refusal: "instrument giantpopstar_theme" */
    public static function label(string $key): string
    {
        $parsed = EntityKey::parse($key);
        return $parsed->kind . ' ' . ($parsed->id ?? $key);
    }

    private static function sameFiles(mixed $a, mixed $b): bool
    {
        if (!is_array($a) || !is_array($b)) return $a === $b;
        ksort($a);
        ksort($b);
        return $a === $b;
    }

    private static function rank(string $key): int
    {
        $i = array_search(explode(':', $key, 2)[0], self::RANK, true);
        return $i === false ? count(self::RANK) : $i;
    }

    /**
     * The catalog modules a graph names: every `kind: "module:<id>"` in it
     * @param array<mixed> $data
     * @return list<string>
     */
    private static function moduleKinds(array $data): array
    {
        $ids = [];
        array_walk_recursive($data, function (mixed $value, int|string $field) use (&$ids): void {
            if ($field === 'kind' && is_string($value) && preg_match('#^module:([a-z0-9][a-z0-9_-]{0,31})$#', $value, $m)) $ids[$m[1]] = true;
        });
        return array_map('strval', array_keys($ids));
    }
}
