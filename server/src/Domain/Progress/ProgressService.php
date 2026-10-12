<?php
declare(strict_types=1);

namespace Scutoons\Domain\Progress;

use Scutoons\Domain\ApiError;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Database\Database;
use stdClass;

/**
 * The reader's save game (DATA.md "The save game", owner 2026-10-06): ONE state per member under the reserved id
 * `scutoons` – story variables of every toon, the history, every toon's checkpoints (`<toon>/chapter/<n>`) and the map.
 * Two writes MERGE (`mergeSaveGame`, the same as @scutoons/content save-game.ts): the map as sets, checkpoints by uri
 * (the later `updated_at`), history entries per key (larger count, first first, last last); variables and every other
 * field are the newer write's (status "stale" tells the client its write was older; the answer is the merged state).
 *
 * Other ids (`/api/progress/<toon>/`) are the per-toon game states from before 2026-10-06 – kept for older viewer builds
 * (last write wins as before: the client sends `updated_at` ms, the stored record is replaced unless it is newer); the
 * migration 2026-10-06-global-save-game merged them into the save game; the new viewer no longer reads or writes them.
 */
final class ProgressService
{
    public const TOON_PATTERN = '[A-Za-z0-9_-]{1,64}';
    /** The save game's id (agents/MAP.md §2.4, DATA.md "The save game"); no toon may have this id */
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
            $current = $this->db->one('SELECT state, updated_at FROM progress WHERE user_id = ? AND toon_id = ? FOR UPDATE', [$userId, $toonId]);
            if ($toonId === self::WORLD && $current) {
                // the save game: discoveries, checkpoints and the history merge – a write from a device that missed
                // another device's never loses them; variables and everything else: the newer write wins
                $stale = (int) $current['updated_at'] > $updatedAt;
                $stored = json_decode((string) $current['state']);
                $merged = self::mergeSaveGame($stale ? $stored : $state, $stale ? $state : $stored);
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
     * The save game (`scutoons`): `$newer` wins for every field except `map` (mergeMapState), `progress` (checkpoints by
     * uri: the later `updated_at`, finished once = finished) and `history` (per key: the larger `n`, the smaller `first`,
     * the larger `last`; at most HISTORY_LIMIT, the longest-unseen go). A state that is no object counts as empty.
     */
    public static function mergeSaveGame(mixed $newer, mixed $older): stdClass
    {
        $out = $newer instanceof stdClass ? clone $newer : ($older instanceof stdClass ? clone $older : new stdClass());
        $field = fn(mixed $state, string $key): mixed => $state instanceof stdClass ? ($state->{$key} ?? null) : null;
        if ($newer instanceof stdClass && $older instanceof stdClass && !property_exists($newer, 'variables') && property_exists($older, 'variables')) {
            $out->variables = $older->variables;
        }
        $mapA = $field($newer, 'map');
        $mapB = $field($older, 'map');
        if ($mapA !== null || $mapB !== null) $out->map = self::mergeMapState($mapA, $mapB);
        $progressA = $field($newer, 'progress');
        $progressB = $field($older, 'progress');
        if ($progressA !== null || $progressB !== null) $out->progress = self::mergeCheckpoints($progressB, $progressA);
        $historyA = $field($newer, 'history');
        $historyB = $field($older, 'history');
        if ($historyA !== null || $historyB !== null) $out->history = self::mergeHistory($historyA, $historyB);
        if (is_string($field($older, 'created_at')) && is_string($field($newer, 'created_at'))
            && (strtotime((string) $older->created_at) ?: PHP_INT_MAX) < (strtotime((string) $newer->created_at) ?: PHP_INT_MAX)) {
            $out->created_at = $older->created_at;
        }
        return $out;
    }

    /** the most history entries kept (@scutoons/content history.ts HISTORY_LIMIT) */
    public const HISTORY_LIMIT = 1000;

    /**
     * Checkpoints of both (lists of { uri, isComplete, position?, created_at, updated_at }) by uri: the later
     * `updated_at` (a tie: the finished one); finished in either = finished.
     * @return list<stdClass>
     */
    public static function mergeCheckpoints(mixed $a, mixed $b): array
    {
        $out = [];
        foreach ([...(is_array($a) ? $a : []), ...(is_array($b) ? $b : [])] as $c) {
            if (!$c instanceof stdClass || !is_string($c->uri ?? null)) continue;
            $have = $out[$c->uri] ?? null;
            if ($have === null) {
                $out[$c->uri] = $c;
                continue;
            }
            $d = (strtotime((string) ($c->updated_at ?? '')) ?: 0) - (strtotime((string) ($have->updated_at ?? '')) ?: 0);
            $pick = clone ($d > 0 || ($d === 0 && !empty($c->isComplete)) ? $c : $have);
            $pick->isComplete = !empty($have->isComplete) || !empty($c->isComplete);
            $out[$c->uri] = $pick;
        }
        return array_values($out);
    }

    /** History entries ({ n, first, last }) of both per key */
    public static function mergeHistory(mixed $a, mixed $b): stdClass
    {
        $part = fn(mixed $h): array => $h instanceof stdClass ? array_filter(get_object_vars($h), fn($e) => $e instanceof stdClass && is_numeric($e->n ?? null)) : [];
        [$x, $y] = [$part($a), $part($b)];
        $merged = [];
        foreach (array_unique([...array_keys($x), ...array_keys($y)]) as $key) {
            $p = $x[$key] ?? null;
            $q = $y[$key] ?? null;
            $only = $p === null ? $q : ($q === null ? $p : null);
            if ($only instanceof stdClass) {
                $merged[$key] = clone $only;
                continue;
            }
            if ($p === null || $q === null) continue;
            $e = new stdClass();
            $e->n = max($p->n, $q->n);
            $e->first = min($p->first ?? $p->last ?? 0, $q->first ?? $q->last ?? 0);
            $e->last = max($p->last ?? 0, $q->last ?? 0);
            $merged[$key] = $e;
        }
        if (count($merged) > self::HISTORY_LIMIT) {
            uasort($merged, fn($e, $f) => ($e->last ?? 0) <=> ($f->last ?? 0));
            $merged = array_slice($merged, count($merged) - self::HISTORY_LIMIT, null, true);
        }
        $out = new stdClass();
        foreach ($merged as $key => $e) $out->{(string) $key} = $e;
        return $out;
    }

    /**
     * Two map states as one (@scutoons/content `mergeMapState`, agents/MAP.md §2.4): union of places and routes, the
     * earlier `unlocked` and first `seen` of a place, the larger `revealed` / `travelled` / `seen` index of a route, the
     * newer (`at`) position per toon. A reset (`resetAt`, ms – the account's "Reset the map", owner 2026-10-11): the
     * later one wins; places and positions from before it drop out, and the routes of a state that has not seen it.
     * Commutative and idempotent.
     */
    public static function mergeMapState(mixed $a, mixed $b): stdClass
    {
        $part = fn(mixed $state, string $key): array => $state instanceof stdClass && ($state->{$key} ?? null) instanceof stdClass ? get_object_vars($state->{$key}) : [];
        $num = fn(mixed $entry, string $key): int|float|null => $entry instanceof stdClass && (is_int($entry->{$key} ?? null) || is_float($entry->{$key} ?? null)) ? $entry->{$key} : null;
        $min = fn(int|float|null $x, int|float|null $y): int|float|null => $x === null ? $y : ($y === null ? $x : min($x, $y));
        $max = fn(int|float|null $x, int|float|null $y): int|float|null => $x === null ? $y : ($y === null ? $x : max($x, $y));

        // the reset: drop what came before it (places / positions by time, routes of a state that has not seen it)
        $resetOf = fn(mixed $state): int|float => $state instanceof stdClass ? ($num($state, 'resetAt') ?? 0) : 0;
        $resetAt = max($resetOf($a), $resetOf($b));
        $since = fn(array $entries, string $key): array => $resetAt <= 0 ? $entries : array_filter($entries, fn(mixed $e) => ($num($e, $key) ?? -1) >= $resetAt);

        $out = new stdClass();
        if ($resetAt > 0) $out->resetAt = $resetAt;
        $out->places = new stdClass();
        [$pa, $pb] = [$since($part($a, 'places'), 'unlocked'), $since($part($b, 'places'), 'unlocked')];
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
        [$ra, $rb] = [$resetOf($a) >= $resetAt ? $part($a, 'routes') : [], $resetOf($b) >= $resetAt ? $part($b, 'routes') : []];
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
        [$xa, $xb] = [$since($part($a, 'positions'), 'at'), $since($part($b, 'positions'), 'at')];
        foreach (array_unique([...array_keys($xa), ...array_keys($xb)]) as $toon) {
            $p = ($xa[$toon] ?? null) instanceof stdClass ? $xa[$toon] : null;
            $q = ($xb[$toon] ?? null) instanceof stdClass ? $xb[$toon] : null;
            $pick = $p === null ? $q : ($q === null ? $p : (($num($q, 'at') ?? 0) > ($num($p, 'at') ?? 0) ? $q : $p));
            if ($pick !== null) $out->positions->{$toon} = $pick;
        }
        // residences (agents/MAP.md §8.10 – who lives in which region): the newer move wins; a reset does not touch them
        $residences = new stdClass();
        [$sa, $sb] = [$part($a, 'residences'), $part($b, 'residences')];
        foreach (array_unique([...array_keys($sa), ...array_keys($sb)]) as $who) {
            $valid = fn(mixed $r) => $r instanceof stdClass && is_string($r->region ?? null) && $r->region !== '' && $num($r, 'at') !== null;
            $p = $valid($sa[$who] ?? null) ? $sa[$who] : null;
            $q = $valid($sb[$who] ?? null) ? $sb[$who] : null;
            $pick = $p === null ? $q : ($q === null ? $p : ($num($q, 'at') > $num($p, 'at') ? $q : $p));
            if ($pick !== null) $residences->{$who} = $pick;
        }
        if (get_object_vars($residences)) $out->residences = $residences;
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
