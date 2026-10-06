<?php
declare(strict_types=1);

namespace Scutoons\Domain\Content\Expr;

/**
 * Expressions in texts (agents/EXPRESSIONS.md §1–2): the function and namespace registry as the server check needs it –
 * names, parameters and types of @scutoons/content `src/expr/registry.ts` (descriptions and implementations live there).
 * ExprTest keeps both equal through shared/content/test/fixtures/expr/registry.json: a function added there is added
 * here too.
 *
 * @phpstan-type Param array{name: string, type: string}
 */
final class Registry
{
    public const FUNCTIONS = [
        ['name' => 'clock', 'params' => [['name' => 'minutes', 'type' => 'number']], 'variadic' => false, 'result' => 'string', 'state' => false],
        ['name' => 'upper', 'params' => [['name' => 'text', 'type' => 'any']], 'variadic' => false, 'result' => 'string', 'state' => false],
        ['name' => 'lower', 'params' => [['name' => 'text', 'type' => 'any']], 'variadic' => false, 'result' => 'string', 'state' => false],
        ['name' => 'capitalize', 'params' => [['name' => 'text', 'type' => 'any']], 'variadic' => false, 'result' => 'string', 'state' => false],
        ['name' => 'number', 'params' => [['name' => 'value', 'type' => 'number']], 'variadic' => false, 'result' => 'string', 'state' => false],
        ['name' => 'min', 'params' => [['name' => 'a', 'type' => 'number'], ['name' => 'b', 'type' => 'number']], 'variadic' => true, 'result' => 'number', 'state' => false],
        ['name' => 'max', 'params' => [['name' => 'a', 'type' => 'number'], ['name' => 'b', 'type' => 'number']], 'variadic' => true, 'result' => 'number', 'state' => false],
        ['name' => 'round', 'params' => [['name' => 'value', 'type' => 'number']], 'variadic' => false, 'result' => 'number', 'state' => false],
        ['name' => 'floor', 'params' => [['name' => 'value', 'type' => 'number']], 'variadic' => false, 'result' => 'number', 'state' => false],
        ['name' => 'random', 'params' => [['name' => 'min', 'type' => 'number'], ['name' => 'max', 'type' => 'number']], 'variadic' => false, 'result' => 'number', 'state' => false],
        ['name' => 'has', 'params' => [['name' => 'item', 'type' => 'string']], 'variadic' => false, 'result' => 'boolean', 'state' => true],
        ['name' => 'count_of', 'params' => [['name' => 'item', 'type' => 'string']], 'variadic' => false, 'result' => 'number', 'state' => true],
        ['name' => 'read_chapter', 'params' => [['name' => 'chapter', 'type' => 'number']], 'variadic' => false, 'result' => 'boolean', 'state' => true],
        ['name' => 'resolved', 'params' => [['name' => 'event', 'type' => 'string']], 'variadic' => false, 'result' => 'boolean', 'state' => true],
        ['name' => 'count', 'params' => [['name' => 'event', 'type' => 'string']], 'variadic' => false, 'result' => 'number', 'state' => true],
    ];

    public const NAMESPACES = [
        ['name' => 'reader', 'members' => [
            ['name' => 'name', 'kind' => 'value', 'type' => 'string', 'params' => []],
            ['name' => 'signedIn', 'kind' => 'value', 'type' => 'boolean', 'params' => []],
            ['name' => 'member', 'kind' => 'value', 'type' => 'boolean', 'params' => []],
            ['name' => 'memberDays', 'kind' => 'value', 'type' => 'number', 'params' => []],
            ['name' => 'since', 'kind' => 'value', 'type' => 'string', 'params' => []],
            ['name' => 'gifts', 'kind' => 'value', 'type' => 'number', 'params' => []],
            ['name' => 'language', 'kind' => 'value', 'type' => 'string', 'params' => []],
            ['name' => 'daysSinceStart', 'kind' => 'value', 'type' => 'number', 'params' => []],
        ]],
        ['name' => 'read', 'members' => [
            ['name' => 'chapter', 'kind' => 'function', 'type' => 'boolean', 'params' => [['name' => 'chapter', 'type' => 'number']]],
            ['name' => 'chapters', 'kind' => 'value', 'type' => 'number', 'params' => []],
            ['name' => 'finished', 'kind' => 'value', 'type' => 'boolean', 'params' => []],
            ['name' => 'position', 'kind' => 'function', 'type' => 'number', 'params' => [['name' => 'chapter', 'type' => 'number']]],
        ]],
        ['name' => 'inventory', 'members' => [
            ['name' => 'has', 'kind' => 'function', 'type' => 'boolean', 'params' => [['name' => 'item', 'type' => 'string']]],
            ['name' => 'count', 'kind' => 'function', 'type' => 'number', 'params' => [['name' => 'item', 'type' => 'string']]],
            ['name' => 'items', 'kind' => 'value', 'type' => 'number', 'params' => []],
            ['name' => 'edition', 'kind' => 'function', 'type' => 'number', 'params' => [['name' => 'item', 'type' => 'string']]],
        ]],
        ['name' => 'avatar', 'members' => [
            ['name' => 'character', 'kind' => 'value', 'type' => 'string', 'params' => []],
            ['name' => 'look', 'kind' => 'value', 'type' => 'string', 'params' => []],
            ['name' => 'wears', 'kind' => 'function', 'type' => 'boolean', 'params' => [['name' => 'item', 'type' => 'string']]],
            ['name' => 'unlocked', 'kind' => 'value', 'type' => 'number', 'params' => []],
        ]],
        ['name' => 'stats', 'members' => [
            ['name' => 'testo', 'kind' => 'value', 'type' => 'number', 'params' => []],
            ['name' => 'appetite', 'kind' => 'value', 'type' => 'number', 'params' => []],
        ]],
        ['name' => 'effects', 'members' => [
            ['name' => 'active', 'kind' => 'function', 'type' => 'boolean', 'params' => [['name' => 'effect', 'type' => 'string']]],
            ['name' => 'count', 'kind' => 'value', 'type' => 'number', 'params' => []],
        ]],
        ['name' => 'map', 'members' => [
            ['name' => 'unlocked', 'kind' => 'value', 'type' => 'number', 'params' => []],
            ['name' => 'seen', 'kind' => 'value', 'type' => 'number', 'params' => []],
            ['name' => 'total', 'kind' => 'value', 'type' => 'number', 'params' => []],
            ['name' => 'place', 'kind' => 'function', 'type' => 'boolean', 'params' => [['name' => 'place', 'type' => 'string']]],
            ['name' => 'here', 'kind' => 'value', 'type' => 'string', 'params' => []],
        ]],
        ['name' => 'now', 'members' => [
            ['name' => 'hour', 'kind' => 'value', 'type' => 'number', 'params' => []],
            ['name' => 'weekday', 'kind' => 'value', 'type' => 'number', 'params' => []],
            ['name' => 'night', 'kind' => 'value', 'type' => 'boolean', 'params' => []],
            ['name' => 'date', 'kind' => 'value', 'type' => 'string', 'params' => []],
        ]],
    ];

    /** @return array<string, array{name: string, params: list<Param>, variadic: bool, result: string, state: bool}> by name */
    public static function functions(): array
    {
        return array_column(self::FUNCTIONS, null, 'name');
    }

    /** @return array<string, list<array{name: string, kind: string, type: string, params: list<Param>}>> namespace → members */
    public static function namespaces(): array
    {
        return array_column(self::NAMESPACES, 'members', 'name');
    }
}
