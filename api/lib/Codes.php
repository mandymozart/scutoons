<?php
declare(strict_types=1);

/**
 * QR codes → items (ITEMS.md "Codes → items"): a code contains its `code_items` rows; without rows but with an
 * edition, the edition's catalog items are the options (the claimer picks item + variant). One code, one user.
 */
final class Codes
{
    /** Codes whose type can be claimed into an inventory (event/xr/page just redirect) */
    public const CLAIMABLE_TYPES = ['membership', 'promo'];

    /** A syntactically valid code (nanoid, max 8 chars) or null */
    public static function normalize(mixed $value): ?string
    {
        if (!is_string($value)) return null;
        $code = trim($value);
        return preg_match('/^[A-Za-z0-9_-]{1,8}$/', $code) ? $code : null;
    }

    /** @return array<string, mixed>|null */
    public static function find(string $code): ?array
    {
        return Db::one('SELECT * FROM qr WHERE code = ?', [$code]);
    }

    /** @return list<array{item_id: string, variant: ?string, qty: int}> the fixed contents (code_items) */
    public static function items(string $code): array
    {
        $rows = Db::all('SELECT item_id, variant, qty FROM code_items WHERE code = ? ORDER BY id', [$code]);
        return array_map(fn($r) => ['item_id' => $r['item_id'], 'variant' => $r['variant'], 'qty' => (int) $r['qty']], $rows);
    }

    /**
     * The items to choose from (only when the code has no code_items but an edition)
     * @return list<array{item_id: string, variants: string[]}>
     */
    public static function options(array $row, ?array $fixedItems = null): array
    {
        $fixedItems ??= self::items($row['code']);
        if ($fixedItems || empty($row['edition_id'])) return [];
        return array_map(fn($item) => ['item_id' => $item['id'], 'variants' => Catalog::variantIds($item)],
            Catalog::itemsForEdition((string) $row['edition_id']));
    }

    /** @return array{id: string, name: string, number: ?int, count: ?int}|null */
    public static function edition(array $row): ?array
    {
        if (empty($row['edition_id'])) return null;
        $edition = Catalog::edition((string) $row['edition_id']);
        return [
            'id' => (string) $row['edition_id'],
            'name' => (string) ($edition['name'] ?? $row['edition_id']),
            'number' => $row['edition_number'] !== null ? (int) $row['edition_number'] : null,
            'count' => isset($edition['edition_count']) ? (int) $edition['edition_count'] : null,
        ];
    }

    public static function isClaimed(array $row): bool
    {
        return $row['claimed_by'] !== null || (string) $row['email'] !== '';
    }

    /** GET /code/?c=CODE → public info about a code (never an email) */
    public static function get(): never
    {
        $code = self::normalize($_GET['c'] ?? $_GET['code'] ?? null);
        if ($code === null) throw new ApiError(400, 'invalid_code', 'Code missing or malformed.');
        $row = self::find($code);
        if (!$row) throw new ApiError(404, 'not_found', 'This code does not exist.');
        if (qrIsExpired($row)) throw new ApiError(410, 'expired', 'This code has expired.');

        $items = self::items($code);
        $response = [
            'code' => $row['code'],
            'type' => $row['type'],
            'claimed' => self::isClaimed($row),
        ];
        $user = Auth::optionalUser();
        if ($user !== null) $response['claimed_by_me'] = $row['claimed_by'] === $user['id'];
        $response += [
            'items' => $items,
            'options' => self::options($row, $items),
            'edition' => self::edition($row),
            'redirect_to' => $row['redirect_to'],
            'expires_at' => $row['expires_at'] ? date('c', (int) strtotime((string) $row['expires_at'])) : null,
        ];
        Http::json(200, $response);
    }

    /**
     * Claims a code for a user: adds its items (or the chosen option) to the inventory and binds the code.
     * Same user again → no change (idempotent). Returns true when the code was newly claimed.
     * Throws 404 not_found, 410 expired, 422 not_claimable, 409 already_claimed, 422 choose_item / invalid_item.
     */
    public static function claim(array $user, string $code, ?string $itemId, ?string $variant): bool
    {
        $code = self::normalize($code) ?? throw new ApiError(400, 'invalid_code', 'Code missing or malformed.');
        return Db::transaction(function () use ($user, $code, $itemId, $variant) {
            $row = Db::one('SELECT * FROM qr WHERE code = ?' . Db::forUpdate(), [$code]);
            if (!$row) throw new ApiError(404, 'not_found', 'This code does not exist.');
            if (qrIsExpired($row)) throw new ApiError(410, 'expired', 'This code has expired.');
            if ($row['claimed_by'] === $user['id']) return false;
            if (!in_array($row['type'], self::CLAIMABLE_TYPES, true)) {
                throw new ApiError(422, 'not_claimable', 'This code cannot be added to an inventory.');
            }
            // Claimed by someone else – or by the old email-only form with another address
            $legacyEmail = strtolower(trim((string) $row['email']));
            if ($row['claimed_by'] !== null || ($legacyEmail !== '' && $legacyEmail !== $user['email'])) {
                throw new ApiError(409, 'already_claimed', 'This code is already in someone else\'s inventory.');
            }

            $grants = self::resolveItems($row, $itemId, $variant);

            $bound = Db::run('UPDATE qr SET claimed_by = ?, email = ?, claimed_at = ? WHERE code = ? AND claimed_by IS NULL',
                [$user['id'], $user['email'], qrNow(), $code]);
            if ($bound->rowCount() !== 1) throw new ApiError(409, 'already_claimed', 'This code is already in someone else\'s inventory.');

            $editionNumber = $row['edition_number'] !== null ? (int) $row['edition_number'] : null;
            foreach ($grants as $grant) {
                Inventory::add($user['id'], $grant['item_id'], $grant['variant'], $grant['qty'], 'qr', $code, $editionNumber);
            }
            return true;
        });
    }

    /**
     * What a claim adds: the code_items, or the one option chosen (auto-picked when there is only one)
     * @return list<array{item_id: string, variant: ?string, qty: int}>
     */
    private static function resolveItems(array $row, ?string $itemId, ?string $variant): array
    {
        $fixed = self::items($row['code']);
        if ($fixed) {
            return array_values(array_filter($fixed, function ($grant) use ($row) {
                if (Catalog::item($grant['item_id'])) return true;
                error_log("[qr-api] code {$row['code']}: unknown item {$grant['item_id']} in code_items – skipped");
                return false;
            }));
        }
        $options = self::options($row, $fixed);
        if (!$options) return []; // e.g. a membership code without edition: claimed, nothing added

        $choices = [];
        foreach ($options as $option) {
            foreach ($option['variants'] ?: [null] as $v) $choices[] = [$option['item_id'], $v];
        }
        if ($itemId === null || $itemId === '') {
            if (count($choices) === 1) return [['item_id' => $choices[0][0], 'variant' => $choices[0][1], 'qty' => 1]];
            throw new ApiError(422, 'choose_item', 'Choose which item this code is for.', ['options' => $options]);
        }
        foreach ($choices as [$choiceItem, $choiceVariant]) {
            if ($choiceItem !== $itemId) continue;
            // An item with a single variant needs no explicit variant
            $variants = array_values(array_filter($choices, fn($c) => $c[0] === $itemId));
            if (($variant === null || $variant === '') && count($variants) === 1) {
                return [['item_id' => $itemId, 'variant' => $variants[0][1], 'qty' => 1]];
            }
            if ($choiceVariant === $variant) return [['item_id' => $itemId, 'variant' => $variant, 'qty' => 1]];
        }
        throw new ApiError(422, 'invalid_item', 'This item or variant is not an option for this code.', ['options' => $options]);
    }
}
