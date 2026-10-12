<?php
declare(strict_types=1);

namespace Scutoons\Domain\Inventory;

use RuntimeException;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Catalog\Catalog;
use Scutoons\Domain\Catalog\ItemRespawn;
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
        private readonly ItemRespawn $respawn,
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
        // the collection limit (`max`, default 99 per account; owner 2026-10-02) – never for a claimed physical
        // product (a code): what was bought is always added
        if ($code === null) $qty = min($qty, $this->room($userId, $item));
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

    /**
     * The default wearables the member owns none of (agents/ITEMS.md "Wardrobe"): starter items with an avatar slot
     * (SCUTOONS T-shirt, swim pants, shorts). Wearables are never consumed, so "owns none" means the account is older
     * than the item.
     * @return list<array<string, mixed>>
     */
    public function missingStarterWearables(string $userId): array
    {
        return array_values(array_filter($this->catalog->starters(), fn(array $item) => is_string($item['slot'] ?? null) && $this->count($userId, (string) $item['id']) === 0));
    }

    /**
     * Grants the missing default wearables (source `starter`); putting the basic dress on is AvatarService::wearBasicDress.
     * No own transaction: signup calls it inside its own, GET /me/ wraps it (AvatarService::ensureWardrobe).
     * @return list<string> the granted item ids
     */
    public function grantStarterWearables(string $userId): array
    {
        $granted = [];
        foreach ($this->missingStarterWearables($userId) as $item) {
            $this->add($userId, (string) $item['id'], null, 1, 'starter');
            $granted[] = (string) $item['id'];
        }
        return $granted;
    }

    /** Default collection limit: at most this many of one item per account (owner 2026-10-02). */
    public const MAX_PER_ACCOUNT = 99;

    /**
     * How many of an item an account may hold: the item's `max`, else MAX_PER_ACCOUNT.
     * @param array<string, mixed> $item
     */
    public static function maxOf(array $item): int
    {
        return max(1, (int) ($item['max'] ?? self::MAX_PER_ACCOUNT));
    }

    /**
     * How many more of an item fit (0 = the collection limit is reached).
     * @param array<string, mixed> $item
     */
    public function room(string $userId, array $item): int
    {
        return max(0, self::maxOf($item) - $this->count($userId, (string) $item['id']));
    }

    /**
     * The world limit (`max_world`, BACKLOG 10.39 / ITEMS.md "Item limits"): how many accounts may own the item at once
     * (1 = it exists only once). An account that already owns it is within it; anyone else gets 409 `taken` while that
     * many accounts own it. Checked inside the grant / craft transaction.
     * @param array<string, mixed> $item
     */
    public function guardWorld(string $userId, array $item): void
    {
        if (!$this->worldFull($userId, $item)) return;
        throw new ApiError(409, 'taken', 'Someone else owns this – it exists only ' . self::worldMax($item) . ' time(s).', ['max_world' => self::worldMax($item)]);
    }

    /** @param array<string, mixed> $item */
    private static function worldMax(array $item): ?int
    {
        $max = $item['max_world'] ?? null;
        return is_int($max) && $max >= 1 ? $max : null;
    }

    /** @param array<string, mixed> $item */
    private function worldFull(string $userId, array $item): bool
    {
        $max = self::worldMax($item);
        if ($max === null || $this->count($userId, (string) $item['id']) > 0) return false;
        return (int) $this->db->value('SELECT COUNT(DISTINCT user_id) FROM inventory WHERE item_id = ? AND qty > 0', [$item['id']]) >= $max;
    }

    public function count(string $userId, string $itemId): int
    {
        return (int) $this->db->value('SELECT COALESCE(SUM(qty), 0) FROM inventory WHERE user_id = ? AND item_id = ?', [$userId, $itemId]);
    }

    /** What `remove` may consume: the pieces without a code (a code-bound piece is physical – it never disappears) */
    public function consumable(string $userId, string $itemId): int
    {
        return (int) $this->db->value('SELECT COALESCE(SUM(qty), 0) FROM inventory WHERE user_id = ? AND item_id = ? AND code IS NULL', [$userId, $itemId]);
    }

    /**
     * Consumes `$qty` of an item (oldest first); unequips removed entries. Never a code-bound piece (owner 2026-10-12:
     * "physische Items, kombiniert, verschwinden nicht" – its row is what `qr.claimed_by` stands for).
     */
    public function remove(string $userId, string $itemId, int $qty): void
    {
        $rows = $this->db->all('SELECT id, qty FROM inventory WHERE user_id = ? AND item_id = ? AND qty > 0 AND code IS NULL ORDER BY acquired_at, id', [$userId, $itemId]);
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
     * Crafts a recipe: the inputs are consumed, the output is added. A physical input (catalog `kind: physical`, or a
     * code-bound piece of any item) only has to be there – it stays in the inventory, the output comes in addition
     * (owner 2026-10-12). 404 unknown_recipe, 422 missing_inputs { missing }.
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
            $output = $this->catalog->item($recipe['output']['item']);
            if ($output && $this->room($userId, $output) < (int) $recipe['output']['qty']) {
                throw new ApiError(409, 'max_reached', 'You cannot hold more of this.', ['max' => self::maxOf($output)]);
            }
            if ($output) $this->guardWorld($userId, $output);
            foreach ($recipe['inputs'] as $input) {
                if (($this->catalog->item($input['item'])['kind'] ?? null) === 'physical') continue;
                $consume = min((int) $input['qty'], $this->consumable($userId, $input['item']));
                if ($consume > 0) $this->remove($userId, $input['item'], $consume);
            }
            $this->add($userId, $recipe['output']['item'], null, (int) $recipe['output']['qty'], 'craft');
        });
        return $recipe;
    }

    /**
     * A story takes an item (VN takeItem / a choice's take – owner 2026-10-07, BACKLOG 10.38): { item_id, qty?, ref? } →
     * the pieces taken, all or nothing: 0 = too few (nothing taken) or this `ref` took before (the client's repeat rule
     * puts the period into the ref; none = every time). Only an item the catalog marks `takeable`, never a physical one
     * (claims move those): 403 not_takeable; 404 unknown_item; 400 invalid_qty (1 … 10).
     * @param array<string, mixed> $body
     * @return array{taken: int, reason: ?string}
     */
    public function take(string $userId, array $body): array
    {
        $itemId = is_string($body['item_id'] ?? null) ? $body['item_id'] : '';
        $item = $this->catalog->item($itemId) ?? throw new ApiError(404, 'unknown_item', 'Unknown item.');
        if (($item['kind'] ?? null) === 'physical' || ($item['takeable'] ?? false) !== true) {
            throw new ApiError(403, 'not_takeable', 'A story cannot take this item.');
        }
        $qty = $body['qty'] ?? 1;
        if (!is_int($qty) || $qty < 1 || $qty > 10) throw new ApiError(400, 'invalid_qty', 'qty is a whole number 1 … 10.');
        $ref = is_string($body['ref'] ?? null) && $body['ref'] !== '' ? substr($body['ref'], 0, 191) : null;
        return $this->db->transaction(function () use ($userId, $item, $qty, $ref) {
            $refBucket = "takeref:$userId:{$item['id']}";
            if ($ref !== null && $this->limits->exists($refBucket, $ref)) return ['taken' => 0, 'reason' => 'already_taken'];
            if ($this->consumable($userId, $item['id']) < $qty) return ['taken' => 0, 'reason' => 'too_few'];
            $this->remove($userId, $item['id'], $qty);
            if ($ref !== null) $this->limits->hit($refBucket, $ref);
            return ['taken' => $qty, 'reason' => null];
        });
    }

    /**
     * Grants a found item: { item_id, source: "story"|"scan", ref?, respawn?, visit? } → 1 granted, 0 = repeated ref.
     * `respawn` (a placed item that reappears, Catalog\ItemRespawn): the interval is checked against the item's range and
     * the ref gets the current period (time bucket / `visit`), so the placement grants once per period – 422
     * respawn_not_allowed / respawn_out_of_range, 400 invalid_respawn / visit_required / ref_required.
     * 403 not_grantable (source not in the item's `grantable`) / no_tool (scan without a tool that detects it),
     * 429 rate_limited (hourly limit) / cooldown (scan tool cooldown) / daily_limit (the camera's `per_day` scans of
     * this item in the last 24 h), 409 max_reached (the collection limit: `max`, default 99), 409 taken (the world limit:
     * `max_world` accounts own it already), 400 invalid_source,
     * 404 unknown_item.
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
        // one of the item's variants (owner 2026-10-08: a colour of the sunglasses); none = the item itself
        $variant = $body['variant'] ?? null;
        if ($variant !== null) {
            $known = array_map(fn($v) => is_array($v) ? ($v['id'] ?? null) : null, is_array($item['variants'] ?? null) ? $item['variants'] : []);
            if (!is_string($variant) || !in_array($variant, $known, true)) throw new ApiError(422, 'unknown_variant', 'This item has no such variant.');
        }
        if (($body['respawn'] ?? null) !== null) {
            if ($ref === null || $source !== 'story') throw new ApiError(400, 'ref_required', 'A respawning story grant needs its ref.');
            $ref = $this->respawn->periodRef($item, $ref, $body['respawn'], $body['visit'] ?? null);
        }

        $granted = $this->db->transaction(function () use ($userId, $item, $source, $ref, $variant) {
            $refBucket = "grantref:$userId:{$item['id']}";
            if ($ref !== null && $this->limits->exists($refBucket, $ref)) return 0;
            if ($this->room($userId, $item) < 1) throw new ApiError(409, 'max_reached', 'You cannot hold more of this.', ['max' => self::maxOf($item)]);
            $this->guardWorld($userId, $item);
            $this->limits->guard("grant:$userId", $this->settings->int('GRANT_MAX_PER_HOUR'), 3600, 'Too many items granted – try again later.');
            if ($source === 'scan') {
                $tools = $this->catalog->scanTools($item['id']);
                $owned = $tools ? array_filter($tools['tools'], fn($tool) => $this->count($userId, $tool) > 0) : [];
                if (!$owned) throw new ApiError(403, 'no_tool', 'You need a tool that can find this item.');
                if ($tools['cooldown'] > 0 && $this->limits->count("scan:$userId:{$item['id']}", $tools['cooldown']) > 0) {
                    throw new ApiError(429, 'cooldown', 'The camera needs a moment.', ['retry_after' => $tools['cooldown']]);
                }
                if ($tools['per_day'] > 0 && $this->limits->count("scanday:$userId:{$item['id']}", 86400) >= $tools['per_day']) {
                    throw new ApiError(429, 'daily_limit', 'Enough for today.', ['per_day' => $tools['per_day']]);
                }
                $this->limits->hit("scan:$userId:{$item['id']}");
                $this->limits->hit("scanday:$userId:{$item['id']}");
            }
            $this->limits->hit("grant:$userId");
            if ($ref !== null) $this->limits->hit($refBucket, $ref);
            $this->add($userId, $item['id'], $variant, 1, $source);
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
                // the guest's variant (a colour) comes along when the item still has it; else the item itself
                $variants = array_map(fn($v) => is_array($v) ? ($v['id'] ?? null) : null, is_array($item['variants'] ?? null) ? $item['variants'] : []);
                $variant = is_array($entry) && is_string($entry['variant'] ?? null) && in_array($entry['variant'], $variants, true) ? $entry['variant'] : null;
                if (empty($item['grantable'])) { $skipped[] = ['item_id' => $itemId, 'reason' => 'not_grantable']; continue; }
                if ($qty < 1) { $skipped[] = ['item_id' => $itemId, 'reason' => 'invalid_qty']; continue; }
                if ($this->worldFull($userId, $item)) { $skipped[] = ['item_id' => $itemId, 'reason' => 'taken']; continue; }
                $qty = min($qty, $max);
                // Non-stackable finds: at most one piece
                if (empty($item['stackable'])) {
                    // one piece per variant (orange and blue sunglasses are two things)
                    $owned = (int) $this->db->value('SELECT COUNT(*) FROM inventory WHERE user_id = ? AND item_id = ? AND ' . ($variant === null ? 'variant IS NULL' : 'variant = ?'), $variant === null ? [$userId, $itemId] : [$userId, $itemId, $variant]);
                    if ($owned > 0) { $skipped[] = ['item_id' => $itemId, 'reason' => 'already_owned']; continue; }
                    $qty = 1;
                }
                $this->add($userId, (string) $itemId, $variant, $qty, $item['grantable'][0]);
                $imported[] = ['item_id' => $itemId, 'qty' => $qty] + ($variant !== null ? ['variant' => $variant] : []);
            }
        });
        return ['imported' => $imported, 'skipped' => $skipped];
    }
}
