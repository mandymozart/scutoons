<?php
declare(strict_types=1);

namespace Scutoons\Domain\Content;

use Scutoons\Domain\ApiError;
use Scutoons\Infrastructure\Database\Database;
use stdClass;

/**
 * The world map's references across its documents (owner 2026-10-12, D7: the server refuses a route to an unknown
 * place – agents/research/MAP-DOCUMENTS.md): a route's place waypoints (`{ location }`) must be places on the map
 * (their newest version, not removed), and a place a route runs through can't be removed or deleted. Judged on the
 * state after the whole save – a batch that adds a place and the route through it, or drops a place and its route
 * waypoint together, is fine. 422 invalid_content { key, problems } otherwise.
 */
final class MapRefs
{
    public function __construct(private readonly Database $db)
    {
    }

    /**
     * @param array<string, mixed> $saves key → the saved document (decoded with stdClass)
     * @param list<string> $deletes keys deleted with them
     */
    public function guard(array $saves, array $deletes = []): void
    {
        $touches = array_filter([...array_keys($saves), ...$deletes], fn(string $k) => str_starts_with($k, 'place:') || str_starts_with($k, 'route:'));
        if ($touches === []) return;
        // the newest documents of every place and route, then this save on top
        $docs = [];
        foreach ($this->db->all("SELECT v.entity_key, v.json FROM entity_versions v JOIN (SELECT MAX(id) AS id FROM entity_versions WHERE kind IN ('place', 'route') GROUP BY entity_key) m ON m.id = v.id") as $r) {
            $docs[(string) $r['entity_key']] = json_decode((string) $r['json']);
        }
        foreach ($saves as $key => $doc) if (str_starts_with($key, 'place:') || str_starts_with($key, 'route:')) $docs[$key] = $doc;
        foreach ($deletes as $key) unset($docs[$key]);
        $live = fn(mixed $doc) => $doc instanceof stdClass && ($doc->removed ?? false) !== true;
        $places = [];
        foreach ($docs as $key => $doc) if (str_starts_with($key, 'place:') && $live($doc)) $places[substr($key, 6)] = true;

        foreach ($docs as $key => $route) {
            if (!str_starts_with($key, 'route:') || !$live($route)) continue;
            $missing = [];
            foreach (is_array($route->waypoints ?? null) ? $route->waypoints : [] as $i => $w) {
                $place = $w instanceof stdClass && is_string($w->location ?? null) ? $w->location : null;
                if ($place !== null && !isset($places[$place])) $missing[] = [$i, $place];
            }
            if ($missing === []) continue;
            // the route is in this save: its own problem; else the save took a place away from under it
            $culprit = isset($saves[$key]) ? $key : $this->culprit($missing, $saves, $deletes) ?? $key;
            $problems = array_map(fn(array $m) => "/waypoints/$m[0]: \"$m[1]\" is no place on the map" . (isset($saves[$key]) ? '' : " (route \"" . substr($key, 6) . "\" runs through it)"), $missing);
            throw new ApiError(422, 'invalid_content', $culprit === $key
                ? "Route \"" . substr($key, 6) . "\" runs through a place that is not on the map: " . implode(', ', array_column($missing, 1)) . '.'
                : "Place \"" . substr($culprit, 6) . "\" is on route \"" . substr($key, 6) . "\" – take it off the route first.", ['key' => $culprit, 'problems' => $problems]);
        }
    }

    /**
     * the place key of this save that made a stored route's waypoint dangle
     * @param list<array{0: int|string, 1: string}> $missing
     * @param array<string, mixed> $saves
     * @param list<string> $deletes
     */
    private function culprit(array $missing, array $saves, array $deletes): ?string
    {
        foreach ($missing as [, $place]) {
            $key = "place:$place";
            if (isset($saves[$key]) || in_array($key, $deletes, true)) return $key;
        }
        return null;
    }
}
