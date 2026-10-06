<?php
declare(strict_types=1);

namespace Scutoons\Console;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Content\EntityKey;

/**
 * The interchange format (agents/DATA.md §4): the ONLY place where entity keys and JSON file names meet. Import and
 * export read/write this folder layout; everything else (API, apps, database) knows entity keys only.
 *
 *   toons.json                         one entry per toon            → toon:<toon>
 *   events.json                                                      → events
 *   scutoons/_/<name>.json             characters, moods, asset-requests, people, roles, map → <name>
 *   <toon>/_/cast.json, credits.json                                 → cast:<toon>, credits:<toon>
 *   <toon>/<l>/toon.json                                             → toon-settings:<toon>:<l>
 *   <toon>/_/season<N>.json, chapter<N>.json                         → season:<toon>:<N>, chapter:<toon>:<N>  (language-neutral, agents/NEUTRAL.md)
 *   <toon>/_/chapter<N>/<name>.json    (a scene)                     → scene:<toon>:<N>:<name>               (language-neutral)
 *   <toon>/<l>/season<N>.json, chapter<N>.json                       → season:<toon>:<N>:<l>, chapter:<toon>:<N>:<l>  (legacy per locale)
 *   <toon>/<l>/chapter<N>/<name>.json  (a scene)                     → scene:<toon>:<N>:<name>:<l>           (legacy per locale)
 *   <toon>/<l>/locations.json, routes.json                           → locations:<toon>:<l>, routes:<toon>:<l>
 *   <toon>/<l>/script.json             (the screenplay)              → script:<toon>:<l>
 * Legacy per-locale characters.json / credits.json (replaced by the registry and the credits entities) are no entities.
 */
final class ContentFiles
{
    public const PLATFORM_DIR = 'scutoons';
    public const NEUTRAL_DIR = '_';
    private const PLATFORM_KINDS = ['characters', 'moods', 'asset-requests', 'people', 'roles', 'map'];

    /** The entity key of a file `<dir>/<locale dir>/<relative>` of the layout, null when it is no entity */
    public static function keyOf(string $dir, string $localeDir, string $relative): ?string
    {
        $name = preg_match('#^([a-z0-9_-]+)\.json$#', $relative, $m) === 1 ? $m[1] : null;
        if ($localeDir === self::NEUTRAL_DIR) {
            if ($dir === self::PLATFORM_DIR) return $name !== null && in_array($name, self::PLATFORM_KINDS, true) ? $name : null;
            if (!preg_match(EntityKey::TOON_PATTERN, $dir)) return null;
            if ($name === 'cast' || $name === 'credits') return "$name:$dir";
            return self::numbered($dir, $relative, null);
        }
        if ($dir === self::PLATFORM_DIR || !preg_match(EntityKey::TOON_PATTERN, $dir) || !preg_match(EntityKey::LOCALE_PATTERN, $localeDir)) return null;
        if ($name === 'toon') return "toon-settings:$dir:$localeDir";
        if ($name === 'locations' || $name === 'routes' || $name === 'script') return "$name:$dir:$localeDir";
        return self::numbered($dir, $relative, $localeDir);
    }

    /** season<N>.json, chapter<N>.json, chapter<N>/<name>.json of a toon → the key (neutral without a locale) */
    private static function numbered(string $dir, string $relative, ?string $locale): ?string
    {
        $suffix = $locale === null ? '' : ":$locale";
        if (preg_match('#^chapter(0|[1-9][0-9]{0,5})/([a-z0-9_-]{1,64})\.json$#', $relative, $s)) return "scene:$dir:$s[1]:$s[2]$suffix";
        if (preg_match('#^(season)([1-9][0-9]{0,5})\.json$#', $relative, $n) || preg_match('#^(chapter)(0|[1-9][0-9]{0,5})\.json$#', $relative, $n)) return "$n[1]:$dir:$n[2]$suffix";
        return null;
    }

    /** The file of an entity, relative to the layout root (toon index entries: `toons.json`, collected) */
    public static function pathOf(EntityKey $key): string
    {
        return match ($key->kind) {
            'toon' => 'toons.json',
            'events' => 'events.json',
            'cast', 'credits' => "$key->toon/_/$key->kind.json",
            'toon-settings' => "$key->toon/$key->locale/toon.json",
            'season', 'chapter' => "$key->toon/" . ($key->locale ?? self::NEUTRAL_DIR) . "/$key->kind$key->number.json",
            'scene' => "$key->toon/" . ($key->locale ?? self::NEUTRAL_DIR) . "/chapter$key->number/$key->scene.json",
            'locations', 'routes', 'script' => "$key->toon/$key->locale/$key->kind.json",
            default => self::PLATFORM_DIR . "/_/$key->kind.json",
        };
    }

    /**
     * Every entity below a layout root, in import order: `toons.json` (one entry per toon, the file's order = the toons
     * list's order), `events.json`, then every toon and locale folder. Each: [key, source file, label, reader].
     * @return list<array{0: string, 1: string, 2: string, 3: callable(): mixed}>
     */
    public static function sources(string $root): array
    {
        $sources = [];
        if (is_file("$root/toons.json")) {
            $index = json_decode((string) file_get_contents("$root/toons.json"));
            foreach (is_array($index) ? $index : [] as $entry) {
                $id = is_object($entry) && is_string($entry->id ?? null) ? $entry->id : null;
                if ($id === null || !preg_match(EntityKey::TOON_PATTERN, $id) || $id === self::PLATFORM_DIR) continue;
                $sources[] = ["toon:$id", "$root/toons.json", "toon:$id (toons.json)", fn() => $entry];
            }
        }
        if (is_file("$root/events.json")) $sources[] = ['events', "$root/events.json", 'events (events.json)', fn() => self::readJson("$root/events.json")];
        foreach (glob("$root/*", GLOB_ONLYDIR) ?: [] as $dir) {
            foreach (glob("$dir/*", GLOB_ONLYDIR) ?: [] as $localeDir) {
                $files = array_merge(glob("$localeDir/*.json") ?: [], glob("$localeDir/*/*.json") ?: []);
                sort($files);
                foreach ($files as $file) {
                    $relative = substr(str_replace('\\', '/', $file), strlen(str_replace('\\', '/', $localeDir)) + 1);
                    $key = self::keyOf(basename($dir), basename($localeDir), $relative);
                    if ($key === null) continue;
                    $sources[] = [$key, $file, $key . ' (' . basename($dir) . '/' . basename($localeDir) . "/$relative)", fn() => self::readJson($file)];
                }
            }
        }
        return $sources;
    }

    /** A JSON file decoded with objects (keeps `{}`), or an ApiError */
    public static function readJson(string $file): mixed
    {
        $decoded = json_decode((string) file_get_contents($file));
        if ($decoded === null && json_last_error() !== JSON_ERROR_NONE) {
            throw new ApiError(422, 'invalid_json', json_last_error_msg());
        }
        return $decoded;
    }
}
