<?php
declare(strict_types=1);

namespace Scutoons\Domain\Progress;

use Scutoons\Domain\ApiError;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Database\Database;
use stdClass;

/**
 * The viewer game state per toon (last write wins): the client sends `updated_at` (ms since 1970) with the state; the
 * stored record is replaced unless it is newer. The answer always carries the record that won, so a client whose
 * write lost (status "stale") can adopt it. The world state `scutoons` (agents/MAP.md §2.4) is merged instead: its `map`
 * unions the discoveries of both writes (status "stale" still tells the client its write was older), the other fields
 * are the newer write's.
 */
final class ProgressService
{
    public const TOON_PATTERN = '[A-Za-z0-9_-]{1,64}';
    /** The world-wide game state (agents/MAP.md §2.4: map unlocks count across every toon); no toon may have this id */
    public const WORLD = 'scutoons';

    public function __construct(
        private readonly Database $db,
        private readonly Clock $clock,
        private readonly Settings $settings,
    ) {
    }

    /** @return array<string, mixed> { toon_id, state, updated_at } (state/updated_at null when nothing is stored) */
    public function get(string $userId, string $toonId): array
    {
        $row = $this->db->one('SELECT state, updated_at FROM progress WHERE user_id = ? AND toon_id = ?', [$userId, $toonId]);
        return ['toon_id' => $toonId] + self::toJson($row);
    }

    /**
     * @param mixed $state      the state decoded with objects (so an empty {} stays an object)
     * @param mixed $updatedAt  ms since 1970 (null = now)
     * @return array<string, mixed> { status: "success"|"stale", toon_id, state, updated_at }
     */
    public function put(string $userId, string $toonId, mixed $state, mixed $updatedAt): array
    {
        if (!$state instanceof stdClass && !is_array($state)) throw new ApiError(400, 'invalid_state', 'state must be a JSON object.');
        $json = json_encode($state, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
        if ($json === false || strlen($json) > $this->settings->int('PROGRESS_MAX_BYTES')) {
            throw new ApiError(413, 'state_too_large', 'The progress state is too large.');
        }
        if ($updatedAt === null) $updatedAt = $this->clock->ms();
        if (!is_int($updatedAt) || $updatedAt < 0) throw new ApiError(400, 'invalid_updated_at', 'updated_at must be milliseconds since 1970.');

        $result = $this->db->transaction(function () use ($userId, $toonId, $json, $updatedAt, $state) {
            $current = $this->db->one('SELECT state, updated_at FROM progress WHERE user_id = ? AND toon_id = ?' . $this->db->forUpdate(), [$userId, $toonId]);
            if ($toonId === self::WORLD && $current) {
                // the world state: the map's discoveries merge as sets (agents/MAP.md §2.4) – a write from a device that
                // missed another device's unlocks never loses them; everything else: the newer write wins
                $stale = (int) $current['updated_at'] > $updatedAt;
                $stored = json_decode((string) $current['state']);
                $merged = self::mergeWorldState($stale ? $stored : $state, $stale ? $state : $stored);
                $mergedJson = (string) json_encode($merged, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
                $time = max((int) $current['updated_at'], $updatedAt);
                $this->db->run('UPDATE progress SET state = ?, updated_at = ? WHERE user_id = ? AND toon_id = ?', [$mergedJson, $time, $userId, $toonId]);
                return ['status' => $stale ? 'stale' : 'success'] + self::toJson(['state' => $mergedJson, 'updated_at' => $time]);
            }
            if ($current && (int) $current['updated_at'] > $updatedAt) {
                return ['status' => 'stale'] + self::toJson($current);
            }
            if ($current) {
                $this->db->run('UPDATE progress SET state = ?, updated_at = ? WHERE user_id = ? AND toon_id = ?', [$json, $updatedAt, $userId, $toonId]);
            } else {
                $this->db->run('INSERT INTO progress (user_id, toon_id, state, updated_at) VALUES (?, ?, ?, ?)', [$userId, $toonId, $json, $updatedAt]);
            }
            return ['status' => 'success'] + self::toJson(['state' => $json, 'updated_at' => $updatedAt]);
        });
        return ['status' => $result['status'], 'toon_id' => $toonId, 'state' => $result['state'], 'updated_at' => $result['updated_at']];
    }

    /**
     * The world state (`scutoons`): `$newer` wins for every field except `map`, whose places / routes / positions merge
     * (mergeMapState). A state that is no object counts as empty.
     */
    public static function mergeWorldState(mixed $newer, mixed $older): stdClass
    {
        $out = $newer instanceof stdClass ? clone $newer : ($older instanceof stdClass ? clone $older : new stdClass());
        $mapA = $newer instanceof stdClass ? ($newer->map ?? null) : null;
        $mapB = $older instanceof stdClass ? ($older->map ?? null) : null;
        if ($mapA !== null || $mapB !== null) $out->map = self::mergeMapState($mapA, $mapB);
        return $out;
    }

    /**
     * Two map states as one (@scutoons/content `mergeMapState`, agents/MAP.md §2.4): union of places and routes, the
     * earlier `unlocked` and first `seen` of a place, the larger `revealed` / `travelled` / `seen` index of a route, the
     * newer (`at`) position per toon. Commutative and idempotent.
     */
    public static function mergeMapState(mixed $a, mixed $b): stdClass
    {
        $part = fn(mixed $state, string $key): array => $state instanceof stdClass && ($state->{$key} ?? null) instanceof stdClass ? get_object_vars($state->{$key}) : [];
        $num = fn(mixed $entry, string $key): int|float|null => $entry instanceof stdClass && (is_int($entry->{$key} ?? null) || is_float($entry->{$key} ?? null)) ? $entry->{$key} : null;
        $min = fn(int|float|null $x, int|float|null $y): int|float|null => $x === null ? $y : ($y === null ? $x : min($x, $y));
        $max = fn(int|float|null $x, int|float|null $y): int|float|null => $x === null ? $y : ($y === null ? $x : max($x, $y));

        $out = new stdClass();
        $out->places = new stdClass();
        [$pa, $pb] = [$part($a, 'places'), $part($b, 'places')];
        foreach (array_unique([...array_keys($pa), ...array_keys($pb)]) as $id) {
            $unlocked = $min($num($pa[$id] ?? null, 'unlocked'), $num($pb[$id] ?? null, 'unlocked'));
            if ($unlocked === null) continue;
            $place = new stdClass();
            $place->unlocked = $unlocked;
            $seen = $min($num($pa[$id] ?? null, 'seen'), $num($pb[$id] ?? null, 'seen'));
            if ($seen !== null) $place->seen = $seen;
            $out->places->{$id} = $place;
        }
        $out->routes = new stdClass();
        [$ra, $rb] = [$part($a, 'routes'), $part($b, 'routes')];
        foreach (array_unique([...array_keys($ra), ...array_keys($rb)]) as $id) {
            $revealed = $max($num($ra[$id] ?? null, 'revealed'), $num($rb[$id] ?? null, 'revealed'));
            if ($revealed === null) continue;
            $route = new stdClass();
            $route->revealed = $revealed;
            foreach (['travelled', 'seen'] as $key) {
                $value = $max($num($ra[$id] ?? null, $key), $num($rb[$id] ?? null, $key));
                if ($value !== null) $route->{$key} = $value;
            }
            $out->routes->{$id} = $route;
        }
        $out->positions = new stdClass();
        [$xa, $xb] = [$part($a, 'positions'), $part($b, 'positions')];
        foreach (array_unique([...array_keys($xa), ...array_keys($xb)]) as $toon) {
            $p = ($xa[$toon] ?? null) instanceof stdClass ? $xa[$toon] : null;
            $q = ($xb[$toon] ?? null) instanceof stdClass ? $xb[$toon] : null;
            $pick = $p === null ? $q : ($q === null ? $p : (($num($q, 'at') ?? 0) > ($num($p, 'at') ?? 0) ? $q : $p));
            if ($pick !== null) $out->positions->{$toon} = $pick;
        }
        return $out;
    }

    /**
     * @param array<string, mixed>|null $row
     * @return array{state: mixed, updated_at: ?int}
     */
    private static function toJson(?array $row): array
    {
        return $row
            ? ['state' => json_decode((string) $row['state']), 'updated_at' => (int) $row['updated_at']]
            : ['state' => null, 'updated_at' => null];
    }
}
