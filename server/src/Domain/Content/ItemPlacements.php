<?php
declare(strict_types=1);

namespace Scutoons\Domain\Content;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Catalog\Catalog;
use Scutoons\Infrastructure\Database\Database;
use stdClass;

/**
 * Item limits across the whole system (BACKLOG 10.39 step 7, agents/ITEMS.md "Item limits"): a catalog item's
 *   `placements` – how often it may be placed on slices, in every toon together (chapters + scenes, all authors);
 *   `max_world`  – how many accounts may own it at once (1 = it exists only once in the world; InventoryService).
 * A chapter / scene counts with the newer of its newest version and its published one (the most placements of the
 * two: a draft that removes one doesn't free it before it is published). Only language-neutral documents count.
 *   GET /api/catalog/items/<id>/placements → where it is placed (the creator: its Problems error, a paste's refusal)
 *   PUT /api/entities/<key> → 422 item_limit when a save adds placements over the limit (a document already over it
 *   still saves as long as it doesn't add one – a lowered limit never blocks unrelated work)
 */
final class ItemPlacements
{
    public function __construct(private readonly Database $db, private readonly Catalog $catalog)
    {
    }

    /**
     * Placements per catalog item in a chapter / scene document.
     * @return array<string, int> item id => count
     */
    public static function count(mixed $content): array
    {
        $out = [];
        $doc = $content instanceof stdClass ? $content : (is_array($content) ? json_decode((string) json_encode($content)) : null);
        if (!$doc instanceof stdClass) return $out;
        $groups = is_array($doc->groups ?? null) ? $doc->groups : [$doc];
        foreach ($groups as $group) {
            if (!$group instanceof stdClass || !is_array($group->slices ?? null)) continue;
            foreach ($group->slices as $slice) {
                if (!$slice instanceof stdClass || !is_array($slice->items ?? null)) continue;
                foreach ($slice->items as $item) {
                    if ($item instanceof stdClass && is_string($item->item ?? null) && $item->item !== '') $out[$item->item] = ($out[$item->item] ?? 0) + 1;
                }
            }
        }
        return $out;
    }

    /**
     * The limits of a catalog item (null: no such item; a limit null: none).
     * @return array{placements: ?int, max_world: ?int}|null
     */
    public function limits(string $itemId): ?array
    {
        $item = $this->catalog->item($itemId);
        if (!$item) return null;
        $limit = fn(mixed $v) => is_int($v) && $v >= 1 ? $v : null;
        return ['placements' => $limit($item['placements'] ?? null), 'max_world' => $limit($item['max_world'] ?? null)];
    }

    /**
     * Where an item is placed: per chapter / scene (neutral documents) its count.
     * @return list<array{key: string, toon: ?string, count: int}>
     */
    public function where(string $itemId): array
    {
        $rows = $this->db->all(
            "SELECT v.entity_key, v.toon, v.json FROM entity_versions v
             WHERE v.kind IN ('chapter', 'scene') AND v.json LIKE ? ESCAPE '!'
               AND (v.status = 'published' OR v.id IN (SELECT MAX(n.id) FROM entity_versions n WHERE n.kind IN ('chapter', 'scene') GROUP BY n.entity_key))",
            [Database::likeContains('"item":' . json_encode($itemId, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE))]
        );
        $out = [];
        foreach ($rows as $row) {
            $key = (string) $row['entity_key'];
            try {
                if (!EntityKey::parse($key)->isNeutralDoc()) continue;
            } catch (\Throwable) {
                continue;
            }
            $n = self::count(json_decode((string) $row['json']))[$itemId] ?? 0;
            if ($n > ($out[$key]['count'] ?? 0)) $out[$key] = ['key' => $key, 'toon' => $row['toon'] !== null ? (string) $row['toon'] : null, 'count' => $n];
        }
        ksort($out);
        return array_values($out);
    }

    /** accounts that own the item now */
    public function owners(string $itemId): int
    {
        return (int) $this->db->value('SELECT COUNT(DISTINCT user_id) FROM inventory WHERE item_id = ? AND qty > 0', [$itemId]);
    }

    /**
     * GET /api/catalog/items/<id>/placements
     * @return array<string, mixed>
     */
    public function report(string $itemId): array
    {
        $limits = $this->limits($itemId) ?? throw new ApiError(404, 'unknown_item', 'Unknown item.');
        $where = $this->where($itemId);
        return [
            'status' => 'success',
            'item' => $itemId,
            'limits' => $limits,
            'placements' => $where,
            'total' => array_sum(array_column($where, 'count')),
            'owners' => $this->owners($itemId),
        ];
    }

    /**
     * The save's problems: an item placed more often than its `placements` allow, counted over every toon – only when
     * this save adds placements of it.
     * @return list<string> "<item>: …"
     */
    public function problems(EntityKey $key, mixed $content): array
    {
        if (!$key->isNeutralDoc() || !in_array($key->kind, ['chapter', 'scene'], true)) return [];
        $out = [];
        foreach (self::count($content) as $itemId => $mine) {
            $limit = $this->limits((string) $itemId)['placements'] ?? null;
            if ($limit === null) continue;
            $here = (string) $key;
            $stored = 0;
            $others = [];
            foreach ($this->where((string) $itemId) as $place) {
                if ($place['key'] === $here) $stored = $place['count'];
                else $others[] = $place;
            }
            $total = $mine + array_sum(array_column($others, 'count'));
            if ($mine <= $stored || $total <= $limit) continue;
            $also = array_map(fn(array $p) => "{$p['key']} ({$p['count']})", $others);
            $out[] = "$itemId: $total placements, the catalog allows $limit" . ($also ? ' – also in ' . implode(', ', $also) : '');
        }
        return $out;
    }

    /** Throws 422 item_limit { problems } when the save adds placements over an item's limit */
    public function guard(EntityKey $key, mixed $content): void
    {
        $problems = $this->problems($key, $content);
        if ($problems) throw new ApiError(422, 'item_limit', 'An item is placed more often than the catalog allows: ' . $problems[0], ['problems' => $problems]);
    }
}
