<?php
declare(strict_types=1);

/**
 * The reader's inventory (ITEMS.md `Entry`). Stackable items are one row per (user, item, variant) with a qty;
 * other items are one row per piece (qty 1), so a piece can be worn on the avatar and keeps its code/edition number.
 */
final class Inventory
{
    /** @return list<array<string, mixed>> the user's entries as JSON */
    public static function entries(string $userId): array
    {
        $rows = Db::all('SELECT * FROM inventory WHERE user_id = ? AND qty > 0 ORDER BY acquired_at, id', [$userId]);
        return array_map([self::class, 'toJson'], $rows);
    }

    /** @param array<string, mixed> $row */
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
            'acquired_at' => Db::iso($row['acquired_at']),
        ];
    }

    /** Adds items (call inside a transaction when combined with other writes) */
    public static function add(string $userId, string $itemId, ?string $variant, int $qty, string $source, ?string $code = null, ?int $editionNumber = null): void
    {
        $item = Catalog::item($itemId) ?? throw new ApiError(404, 'unknown_item', "Unknown item $itemId.");
        if ($qty < 1) return;
        $now = Db::now();
        if (!empty($item['stackable'])) {
            $row = Db::one('SELECT id FROM inventory WHERE user_id = ? AND item_id = ? AND ' . ($variant === null ? 'variant IS NULL' : 'variant = ?') . ' ORDER BY acquired_at LIMIT 1',
                $variant === null ? [$userId, $itemId] : [$userId, $itemId, $variant]);
            if ($row) {
                Db::run('UPDATE inventory SET qty = qty + ?, updated_at = ? WHERE id = ?', [$qty, $now, $row['id']]);
                return;
            }
            Db::run('INSERT INTO inventory (id, user_id, item_id, variant, qty, source, code, edition_number, acquired_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)',
                [Db::uuid(), $userId, $itemId, $variant, $qty, $source, $code, $editionNumber, $now, $now]);
            return;
        }
        for ($i = 0; $i < $qty; $i++) {
            Db::run('INSERT INTO inventory (id, user_id, item_id, variant, qty, source, code, edition_number, acquired_at, updated_at) VALUES (?, ?, ?, ?, 1, ?, ?, ?, ?, ?)',
                [Db::uuid(), $userId, $itemId, $variant, $source, $code, $editionNumber, $now, $now]);
        }
    }

    public static function count(string $userId, string $itemId): int
    {
        return (int) Db::value('SELECT COALESCE(SUM(qty), 0) FROM inventory WHERE user_id = ? AND item_id = ?', [$userId, $itemId]);
    }

    /** Consumes `$qty` of an item (virtual pieces first, oldest first); unequips removed entries */
    public static function remove(string $userId, string $itemId, int $qty): void
    {
        $rows = Db::all('SELECT id, qty FROM inventory WHERE user_id = ? AND item_id = ? AND qty > 0 ORDER BY (code IS NOT NULL), acquired_at, id', [$userId, $itemId]);
        foreach ($rows as $row) {
            if ($qty <= 0) break;
            $take = min($qty, (int) $row['qty']);
            $qty -= $take;
            if ($take >= (int) $row['qty']) {
                Db::run('DELETE FROM inventory WHERE id = ?', [$row['id']]);
                Db::run('DELETE FROM avatar_slots WHERE user_id = ? AND entry_id = ?', [$userId, $row['id']]);
            } else {
                Db::run('UPDATE inventory SET qty = qty - ?, updated_at = ? WHERE id = ?', [$take, Db::now(), $row['id']]);
            }
        }
        if ($qty > 0) throw new RuntimeException("remove: not enough $itemId");
    }

    /** POST /inventory/claim/ { code, item_id?, variant? } → { status, claimed, inventory } */
    public static function claim(array $body): never
    {
        $user = Auth::user();
        $code = Codes::normalize($body['code'] ?? null) ?? throw new ApiError(400, 'invalid_code', 'Code missing or malformed.');
        $itemId = is_string($body['item_id'] ?? null) ? $body['item_id'] : null;
        $variant = is_string($body['variant'] ?? null) ? $body['variant'] : null;
        $new = Codes::claim($user, $code, $itemId, $variant);
        Http::json(200, ['status' => 'success', 'claimed' => $new ? 'new' : 'already_yours', 'code' => $code, 'inventory' => self::entries($user['id'])]);
    }

    /** POST /inventory/craft/ { recipe_id } → { status, inventory } · 422 missing_inputs */
    public static function craft(array $body): never
    {
        $user = Auth::user();
        $recipe = is_string($body['recipe_id'] ?? null) ? Catalog::recipe($body['recipe_id']) : null;
        if (!$recipe) throw new ApiError(404, 'unknown_recipe', 'Unknown recipe.');

        Db::transaction(function () use ($user, $recipe) {
            $missing = [];
            foreach ($recipe['inputs'] as $input) {
                $have = self::count($user['id'], $input['item']);
                if ($have < (int) $input['qty']) $missing[] = ['item' => $input['item'], 'qty' => (int) $input['qty'] - $have];
            }
            if ($missing) throw new ApiError(422, 'missing_inputs', 'Some ingredients are missing.', ['missing' => $missing]);
            foreach ($recipe['inputs'] as $input) self::remove($user['id'], $input['item'], (int) $input['qty']);
            self::add($user['id'], $recipe['output']['item'], null, (int) $recipe['output']['qty'], 'craft');
        });
        Http::json(200, ['status' => 'success', 'recipe_id' => $recipe['id'], 'inventory' => self::entries($user['id'])]);
    }

    /**
     * POST /inventory/grant/ { item_id, source: "story"|"scan", ref? } → { status, granted, inventory }
     * 403 not_grantable (source not in the item's `grantable`) / no_tool (scan without a tool that detects it),
     * 429 rate_limited (hourly limit) / cooldown (scan tool cooldown). A repeated `ref` grants nothing (granted: 0).
     */
    public static function grant(array $body): never
    {
        $user = Auth::user();
        $itemId = is_string($body['item_id'] ?? null) ? $body['item_id'] : '';
        $item = Catalog::item($itemId) ?? throw new ApiError(404, 'unknown_item', 'Unknown item.');
        $source = $body['source'] ?? null;
        if (!in_array($source, ['story', 'scan'], true)) throw new ApiError(400, 'invalid_source', 'source must be "story" or "scan".');
        if (!in_array($source, $item['grantable'] ?? [], true)) {
            throw new ApiError(403, 'not_grantable', "This item cannot be granted by $source.");
        }
        $ref = is_string($body['ref'] ?? null) && $body['ref'] !== '' ? substr($body['ref'], 0, 191) : null;

        $granted = Db::transaction(function () use ($user, $item, $source, $ref) {
            $uid = $user['id'];
            $refBucket = "grantref:$uid:{$item['id']}";
            if ($ref !== null && RateLimit::exists($refBucket, $ref)) return 0;
            RateLimit::guard("grant:$uid", Config::int('GRANT_MAX_PER_HOUR'), 3600, 'Too many items granted – try again later.');
            if ($source === 'scan') {
                $tools = Catalog::scanTools($item['id']);
                $owned = $tools ? array_filter($tools['tools'], fn($tool) => self::count($uid, $tool) > 0) : [];
                if (!$owned) throw new ApiError(403, 'no_tool', 'You need a tool that can find this item.');
                if ($tools['cooldown'] > 0 && RateLimit::count("scan:$uid:{$item['id']}", $tools['cooldown']) > 0) {
                    throw new ApiError(429, 'cooldown', 'The camera needs a moment.', ['retry_after' => $tools['cooldown']]);
                }
                RateLimit::hit("scan:$uid:{$item['id']}");
            }
            RateLimit::hit("grant:$uid");
            if ($ref !== null) RateLimit::hit($refBucket, $ref);
            self::add($uid, $item['id'], null, 1, $source);
            return 1;
        });
        RateLimit::prune();
        Http::json(200, ['status' => 'success', 'granted' => $granted, 'inventory' => self::entries($user['id'])]);
    }

    /**
     * POST /inventory/import/ { items: [{ item_id, qty }] } → { status, imported, skipped, inventory }
     * Merges a guest inventory after login: only items with a non-empty `grantable` (story/scan finds);
     * starters, physical and crafted items are skipped. qty is capped at IMPORT_MAX_QTY per item.
     */
    public static function import(array $body): never
    {
        $user = Auth::user();
        $items = $body['items'] ?? null;
        if (!is_array($items) || !array_is_list($items) || count($items) > 100) {
            throw new ApiError(400, 'invalid_items', 'items must be a list of { item_id, qty }.');
        }
        $imported = [];
        $skipped = [];
        Db::transaction(function () use ($user, $items, &$imported, &$skipped) {
            RateLimit::guard("import:{$user['id']}", Config::int('IMPORT_MAX_PER_HOUR'), 3600, 'Too many imports – try again later.');
            RateLimit::hit("import:{$user['id']}");
            $max = Config::int('IMPORT_MAX_QTY');
            foreach ($items as $entry) {
                $itemId = is_array($entry) && is_string($entry['item_id'] ?? null) ? $entry['item_id'] : null;
                $qty = is_array($entry) && is_int($entry['qty'] ?? null) ? $entry['qty'] : (is_array($entry) && !isset($entry['qty']) ? 1 : 0);
                $item = $itemId !== null ? Catalog::item($itemId) : null;
                if (!$item) { $skipped[] = ['item_id' => $itemId, 'reason' => 'unknown_item']; continue; }
                if (empty($item['grantable'])) { $skipped[] = ['item_id' => $itemId, 'reason' => 'not_grantable']; continue; }
                if ($qty < 1) { $skipped[] = ['item_id' => $itemId, 'reason' => 'invalid_qty']; continue; }
                $qty = min($qty, $max);
                // Non-stackable finds: at most one piece
                if (empty($item['stackable'])) {
                    if (self::count($user['id'], $itemId) > 0) { $skipped[] = ['item_id' => $itemId, 'reason' => 'already_owned']; continue; }
                    $qty = 1;
                }
                self::add($user['id'], $itemId, null, $qty, $item['grantable'][0]);
                $imported[] = ['item_id' => $itemId, 'qty' => $qty];
            }
        });
        Http::json(200, ['status' => 'success', 'imported' => $imported, 'skipped' => $skipped, 'inventory' => self::entries($user['id'])]);
    }

    /**
     * GET /me/ → { user, access, memberships, inventory, avatar }
     * access = permissions of the owned, non-expired codes (api/permissions.json); memberships = owned codes of
     * claimable types (with the permissions each grants).
     */
    public static function me(): never
    {
        $user = Auth::user();
        $codes = Db::all('SELECT code, type, expires_at FROM qr WHERE claimed_by = ? ORDER BY claimed_at, code', [$user['id']]);
        $claimable = array_values(array_filter($codes, fn($r) => Permissions::claimable((string) $r['type'])));
        Http::json(200, [
            'user' => Users::toJson($user),
            'access' => Permissions::access($codes),
            'memberships' => array_map(fn($r) => [
                'code' => $r['code'],
                'type' => $r['type'],
                'expires_at' => $r['expires_at'] ? date('c', (int) strtotime((string) $r['expires_at'])) : null,
                'grants' => Permissions::grants((string) $r['type']),
            ], $claimable),
            'inventory' => self::entries($user['id']),
            'avatar' => Avatar::toJson($user['id']),
        ]);
    }
}
