<?php
declare(strict_types=1);

namespace Scutoons\Domain\Content;

use Scutoons\Domain\Content\Expr\Registry;

/**
 * Story variable domains (owner 2026-10-06; the same as @scutoons/content variables.ts): a name as written is relative
 * to the toon the content is in, its storage key in the save game is absolute –
 * `coins` → `<toon>/coins` (this toon's), `global.coins` → `coins` (every toon's), `toptoy.coins` → `toptoy/coins`.
 */
final class Variables
{
    public const GLOBAL = 'global';
    /** a name as written: `name`, `global.name` or `<toon>.name` */
    public const WRITTEN = '/^(?:([a-z_][a-z0-9_]*)\.)?([A-Za-z_][A-Za-z0-9_]*)$/';

    /** the storage key of a written name, seen from `$toon` */
    public static function key(string $written, string $toon): string
    {
        if (preg_match(self::WRITTEN, $written, $m) === 1) {
            if ($m[1] === self::GLOBAL) return $m[2];
            if ($m[1] !== '' && $m[1] !== $toon) return $m[1] . '/' . $m[2];
            return $toon . '/' . $m[2];
        }
        return $toon . '/' . $written;
    }

    /** a storage key as written in `$toon`'s content (`catpope/x` → `x` in catpope, `catpope.x` elsewhere; `x` → `global.x`) */
    public static function written(string $key, string $toon): string
    {
        $slash = strpos($key, '/');
        if ($slash === false) return self::GLOBAL . '.' . $key;
        $owner = substr($key, 0, $slash);
        $name = substr($key, $slash + 1);
        return $owner === $toon ? $name : $owner . '.' . $name;
    }

    /** whose a storage key is: the owning toon (`catpope/x` → catpope), `global` for a key without one (`x`) */
    public static function scope(string $key): string
    {
        $slash = strpos($key, '/');
        return $slash === false ? self::GLOBAL : substr($key, 0, $slash);
    }

    /** the plain name of a storage key (`toptoy/coins` → `coins`) */
    public static function name(string $key): string
    {
        $slash = strpos($key, '/');
        return $slash === false ? $key : substr($key, $slash + 1);
    }

    /**
     * whether a name an expression reads is a story variable: `coins`, `global.coins`, `toptoy.coins` – not a namespace
     * member (`reader.name`)
     * @param list<string> $path
     */
    public static function isStoryPath(array $path): bool
    {
        $spaces = Registry::namespaces();
        return (count($path) === 1 || count($path) === 2) && !isset($spaces[$path[0]]);
    }
}
