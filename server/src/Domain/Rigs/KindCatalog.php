<?php
declare(strict_types=1);

namespace Scutoons\Domain\Rigs;

/**
 * The explanatory node-kind catalog (#315, agents/research/ENGINE-REFACTOR-PLAN.md §2.4): kinds.json is GENERATED from
 * deck/catalog (`cargo run -p deck-space --features schema --bin deck-schema`) – every built-in kind with its ports
 * (type, direction, channels) and parameters (range, curve, unit, default, doc). GET /api/kinds serves it merged with
 * the descriptors of the published modules (`module:<id>`), which are only known at run time.
 */
final class KindCatalog
{
    public const FILE = __DIR__ . '/kinds.json';

    /** @return array{catalog_version: int, kinds: list<array<string, mixed>>} the built-ins as generated */
    public static function builtin(): array
    {
        $json = json_decode((string) file_get_contents(self::FILE), true, 512, JSON_THROW_ON_ERROR);
        return ['catalog_version' => (int) $json['catalog_version'], 'kinds' => $json['kinds']];
    }

    /**
     * A published module's descriptor (ABI v1: abi, name, version, params, ports with `kind`) in the catalog's shape
     * @param array<string, mixed> $descriptor
     * @return array<string, mixed>
     */
    public static function moduleKind(string $id, array $descriptor): array
    {
        $ports = [];
        foreach ((array) ($descriptor['ports'] ?? []) as $p) {
            $ports[] = ['name' => $p['name'], 'dir' => $p['dir'], 'type' => $p['type'] ?? $p['kind'] ?? 'audio'];
        }
        $params = [];
        foreach ((array) ($descriptor['params'] ?? []) as $p) {
            $params[] = ['name' => $p['name'], 'min' => $p['min'], 'max' => $p['max'], 'default' => $p['default'], 'curve' => 'lin'];
        }
        return [
            'name' => "module:$id",
            'version' => (string) ($descriptor['version'] ?? '1'),
            'domain' => 'audio',
            'doc' => "The published module $id.",
            'feature' => 'modules',
            'scope' => 'graph',
            'abi' => $descriptor['abi'] ?? null,
            'ports' => $ports,
            'params' => $params,
        ];
    }

    /**
     * The built-ins plus the published modules
     * @param array<string, mixed> $modules id → the published module document (the catalog kind `modules`)
     * @return array{catalog_version: int, kinds: list<array<string, mixed>>}
     */
    public static function merged(array $modules): array
    {
        $catalog = self::builtin();
        foreach ($modules as $id => $module) {
            if (is_array($module) && is_array($module['descriptor'] ?? null)) $catalog['kinds'][] = self::moduleKind((string) $id, $module['descriptor']);
        }
        return $catalog;
    }
}
