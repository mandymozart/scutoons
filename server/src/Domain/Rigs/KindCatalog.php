<?php
declare(strict_types=1);

namespace Scutoons\Domain\Rigs;

use Scutoons\Domain\Modules\ModuleSource;

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
     * A published module's descriptor (module ABI v2: deck-catalog's one `Descriptor`, the shape the built-ins have –
     * ports with `type`, channels, event names; parameters with curve, unit, choices, doc) as a kind `module:<id>`
     * @param array<string, mixed> $descriptor
     * @return array<string, mixed>
     */
    public static function moduleKind(string $id, array $descriptor): array
    {
        $params = [];
        foreach ((array) ($descriptor['params'] ?? []) as $p) {
            $params[] = (array) $p + ['curve' => 'lin'];
        }
        $kind = [
            'name' => "module:$id",
            'version' => (string) ($descriptor['version'] ?? '1'),
            'domain' => (string) ($descriptor['domain'] ?? 'audio'),
            'doc' => (string) ($descriptor['doc'] ?? '') !== '' ? (string) $descriptor['doc'] : "The published module $id.",
            'feature' => 'modules',
            'scope' => 'graph',
            'abi' => $descriptor['abi'] ?? null,
            'ports' => array_values((array) ($descriptor['ports'] ?? [])),
            'params' => $params,
        ];
        foreach (['needs', 'wants', 'reads', 'latency', 'assets', 'preset'] as $field) {
            if (array_key_exists($field, $descriptor)) $kind[$field] = $descriptor[$field];
        }
        return $kind;
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
            // a module of another ABI is no kind any deck loads (#343: it is recompiled, never read)
            if (is_array($module) && is_array($module['descriptor'] ?? null) && ($module['descriptor']['abi'] ?? null) === ModuleSource::ABI) {
                $catalog['kinds'][] = self::moduleKind((string) $id, $module['descriptor']);
            }
        }
        return $catalog;
    }
}
