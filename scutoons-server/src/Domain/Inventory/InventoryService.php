<?php
declare(strict_types=1);

namespace Scutoons\Domain\Inventory;

use RuntimeException;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Catalog\Catalog;
use Scutoons\Domain\RateLimit\RateLimiter;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Ids;
use Scutoons\Support\Time;

/**
 * The reader's inventory (agents/ITEMS.md `Entry`). Stackable items are one row per (user, item, variant) with a qty;
 * other items are one row per piece (qty 1), so a piece can be worn on the avatar and keeps its code/edition number.
 */
final class InventoryService
{
    public function __construct(
        private readonly Database $db,
        private readonly Clock $clock,
        private readonly Catalog $catalog,
        private readonly RateLimiter $limits,
        private readonly Settings $settings,
    ) {
    }

    /** @return list<array<string, mixed>> the user's entries as JSON */
    public function entries(string $userId): array
    {
        $rows = $this->db->all('SELECT * FROM inventory WHERE user_id = ? AND qty > 0 ORDER BY acquired_at, id', [$userId]);
        return array_map([self::class, 'toJson'], $rows);
    }

    /**
     * @param array<string, mixed> $row
     * @return array<string, mixed>
     */
    public static function toJson(array $row): array
    {
        return [
            'id' => $row['id'],
            'item_id' => $row['item_id'],
            'variant' => $row['variant'],
            'qty' => (int) $row['qty'],
            'source' => $row['source'],
            'code' => $row['code'],
            'edition_number' => $row['edition_number'] !== null ? (int) $row['edition_number'] : null,
            'acquired_at' => Time::iso($row['acquired_at']),
        ];
    }

    /** Adds items (call inside a transaction when combined with other writes) */
    public function add(string $userId, string $itemId, ?string $variant, int $qty, string $source, ?string $code = null, ?int $editionNumber = null): void
    {
        $item = $this->catalog->item($itemId) ?? throw new ApiError(404, 'unknown_item', "Unknown item $itemId.");
        if ($qty < 1) return;
        $now = $this->clock->utc();
        if (!empty($item['stackable'])) {
            $row = $this->db->one('SELECT id FROM inventory WHERE user_id = ? AND item_id = ? AND ' . ($variant === null ? 'variant IS NULL' : 'variant = ?') . ' ORDER BY acquired_at LIMIT 1',
                $variant === null ? [$userId, $itemId] : [$userId, $itemId, $variant]);
            if ($row) {
                $this->db->run('UPDATE inventory SET qty = qty + ?, updated_at = ? WHERE id = ?', [$qty, $now, $row['id']]);
                return;
            }
            $this->db->run('INSERT INTO inventory (id, user_id, item_id, variant, qty, source, code, edition_number, acquired_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)',
                [Ids::uuid(), $userId, $itemId, $variant, $qty, $source, $code, $editionNumber, $now, $now]);
            return;
        }
        for ($i = 0; $i < $qty; $i++) {
            $this->db->run('INSERT INTO inventory (id, user_id, item_id, variant, qty, source, code, edition_number, acquired_at, updated_at) VALUES (?, ?, ?, ?, 1, ?, ?, ?, ?, ?)',
                [Ids::uuid(), $userId, $itemId, $variant, $source, $code, $editionNumber, $now, $now]);
        }
    }

    public function count(string $userId, string $itemId): int
    {
        return (int) $this->db->value('SELECT COALESCE(SUM(qty), 0) FROM inventory WHERE user_id = ? AND item_id = ?', [$userId, $itemId]);
    }

    /** Consumes `$qty` of an item (virtual pieces first, oldest first); unequips removed entries */
    public function remove(string $userId, string $itemId, int $qty): void
    {
        $rows = $this->db->all('SELECT id, qty FROM inventory WHERE user_id = ? AND item_id = ? AND qty > 0 ORDER BY (code IS NOT NULL), acquired_at, id', [$userId, $itemId]);
        foreach ($rows as $row) {
            if ($qty <= 0) break;
            $take = min($qty, (int) $row['qty']);
            $qty -= $take;
            if ($take >= (int) $row['qty']) {
                $this->db->run('DELETE FROM inventory WHERE id = ?', [$row['id']]);
                $this->db->run('DELETE FROM avatar_slots WHERE user_id = ? AND entry_id = ?', [$userId, $row['id']]);
            } else {
                $this->db->run('UPDATE inventory SET qty = qty - ?, updated_at = ? WHERE id = ?', [$take, $this->clock->utc(), $row['id']]);
            }
        }
        if ($qty > 0) throw new RuntimeException("remove: not enough $itemId");
    }

    /**
     * Crafts a recipe: all inputs are consumed, the output is added. 404 unknown_recipe, 422 missing_inputs { missing }.
     * @return array<string, mixed> the recipe
     */
    public function craft(string $userId, mixed $recipeId): array
    {
        $recipe = is_string($recipeId) ? $this->catalog->recipe($recipeId) : null;
        if (!$recipe) throw new ApiError(404, 'unknown_recipe', 'Unknown recipe.');

        $this->db->transaction(function () use ($userId, $recipe) {
            $missing = [];
            foreach ($recipe['inputs'] as $input) {
                $have = $this->count($userId, $input['item']);
                if ($have < (int) $input['qty']) $missing[] = ['item' => $input['item'], 'qty' => (int) $input['qty'] - $have];
            }
            if ($missing) throw new ApiError(422, 'missing_inputs', 'Some ingredients are missing.', ['missing' => $missing]);
            foreach ($recipe['inputs'] as $input) $this->remove($userId, $input['item'], (int) $input['qty']);
            $this->add($userId, $recipe['output']['item'], null, (int) $recipe['output']['qty'], 'craft');
        });
        return $recipe;
    }

    /**
     * Grants a found item: { item_id, source: "story"|"scan", ref? } → 1 granted, 0 = repeated ref.
     * 403 not_grantable (source not in the item's `grantable`) / no_tool (scan without a tool that detects it),
     * 429 rate_limited (hourly limit) / cooldown (scan tool cooldown), 400 invalid_source, 404 unknown_item.
     * @param array<string, mixed> $body
     */
    public function grant(string $userId, array $body): int
    {
        $itemId = is_string($body['item_id'] ?? null) ? $body['item_id'] : '';
        $item = $this->catalog->item($itemId) ?? throw new ApiError(404, 'unknown_item', 'Unknown item.');
        $source = $body['source'] ?? null;
        if (!in_array($source, ['story', 'scan'], true)) throw new ApiError(400, 'invalid_source', 'source must be "story" or "scan".');
        if (!in_array($source, $item['grantable'] ?? [], true)) {
            throw new ApiError(403, 'not_grantable', "This item cannot be granted by $source.");
        }
        $ref = is_string($body['ref'] ?? null) && $body['ref'] !== '' ? substr($body['ref'], 0, 191) : null;

        $granted = $this->db->transaction(function () use ($userId, $item, $source, $ref) {
            $refBucket = "grantref:$userId:{$item['id']}";
            if ($ref !== null && $this->limits->exists($refBucket, $ref)) return 0;
            $this->limits->guard("grant:$userId", $this->settings->int('GRANT_MAX_PER_HOUR'), 3600, 'Too many items granted – try again later.');
            if ($source === 'scan') {
                $tools = $this->catalog->scanTools($item['id']);
                $owned = $tools ? array_filter($tools['tools'], fn($tool) => $this->count($userId, $tool) > 0) : [];
                if (!$owned) throw new ApiError(403, 'no_tool', 'You need a tool that can find this item.');
                if ($tools['cooldown'] > 0 && $this->limits->count("scan:$userId:{$item['id']}", $tools['cooldown']) > 0) {
                    throw new ApiError(429, 'cooldown', 'The camera needs a moment.', ['retry_after' => $tools['cooldown']]);
                }
                $this->limits->hit("scan:$userId:{$item['id']}");
            }
            $this->limits->hit("grant:$userId");
            if ($ref !== null) $this->limits->hit($refBucket, $ref);
            $this->add($userId, $item['id'], null, 1, $source);
            return 1;
        });
        $this->limits->prune();
        return $granted;
    }

    /**
     * Merges a guest inventory after login: only items with a non-empty `grantable` (story/scan finds); starters,
     * physical and crafted items are skipped. qty is capped at IMPORT_MAX_QTY per item.
     * @param array<string, mixed> $body { items: [{ item_id, qty }] }
     * @return array{imported: list<array<string, mixed>>, skipped: list<array<string, mixed>>}
     */
    public function import(string $userId, array $body): array
    {
        $items = $body['items'] ?? null;
        if (!is_array($items) || !array_is_list($items) || count($items) > 100) {
            throw new ApiError(400, 'invalid_items', 'items must be a list of { item_id, qty }.');
        }
        $imported = [];
        $skipped = [];
        $this->db->transaction(function () use ($userId, $items, &$imported, &$skipped) {
            $this->limits->guard("import:$userId", $this->settings->int('IMPORT_MAX_PER_HOUR'), 3600, 'Too many imports – try again later.');
            $this->limits->hit("import:$userId");
            $max = $this->settings->int('IMPORT_MAX_QTY');
            foreach ($items as $entry) {
                $itemId = is_array($entry) && is_string($entry['item_id'] ?? null) ? $entry['item_id'] : null;
                $qty = is_array($entry) && is_int($entry['qty'] ?? null) ? $entry['qty'] : (is_array($entry) && !isset($entry['qty']) ? 1 : 0);
                $item = $itemId !== null ? $this->catalog->item($itemId) : null;
                if (!$item) { $skipped[] = ['item_id' => $itemId, 'reason' => 'unknown_item']; continue; }
                if (empty($item['grantable'])) { $skipped[] = ['item_id' => $itemId, 'reason' => 'not_grantable']; continue; }
                if ($qty < 1) { $skipped[] = ['item_id' => $itemId, 'reason' => 'invalid_qty']; continue; }
                $qty = min($qty, $max);
                // Non-stackable finds: at most one piece
                if (empty($item['stackable'])) {
                    if ($this->count($userId, (string) $itemId) > 0) { $skipped[] = ['item_id' => $itemId, 'reason' => 'already_owned']; continue; }
                    $qty = 1;
                }
                $this->add($userId, (string) $itemId, null, $qty, $item['grantable'][0]);
                $imported[] = ['item_id' => $itemId, 'qty' => $qty];
            }
        });
        return ['imported' => $imported, 'skipped' => $skipped];
    }
}
