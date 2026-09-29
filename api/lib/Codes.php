<?php
declare(strict_types=1);

/**
 * QR codes → items (ITEMS.md "Codes → items"): a code contains its `code_items` rows; without rows but with an
 * edition, the edition's catalog items are the options (the claimer picks item + variant). One code, one user.
 */
final class Codes
{
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
     * Claims a code for a user (see claimFor). Returns true when the code was newly claimed.
     */
    public static function claim(array $user, string $code, ?string $itemId, ?string $variant): bool
    {
        return self::claimFor($user, $code, $itemId, $variant)[0];
    }

    /**
     * Claims a code: adds its items (or the chosen option) to the inventory and binds the code to the user.
     * `$user` is a user row, or a factory that creates the user – called inside the transaction only after every
     * check passed (so choose_item / already_claimed never leave an orphan account). Same user again → no change.
     * Throws 404 not_found, 410 expired, 403 not_unlocking, 409 already_claimed / email_required,
     * 422 choose_item / invalid_item.
     * @return array{0: bool, 1: array<string, mixed>} [newly claimed, user]
     */
    public static function claimFor(array|Closure $user, string $code, ?string $itemId, ?string $variant): array
    {
        $code = self::normalize($code) ?? throw new ApiError(400, 'invalid_code', 'Code missing or malformed.');
        return Db::transaction(function () use ($user, $code, $itemId, $variant) {
            $row = Db::one('SELECT * FROM qr WHERE code = ?' . Db::forUpdate(), [$code]);
            if (!$row) throw new ApiError(404, 'not_found', 'This code does not exist.');
            if (qrIsExpired($row)) throw new ApiError(410, 'expired', 'This code has expired.');
            if (!Permissions::claimable((string) $row['type'])) {
                throw new ApiError(403, 'not_unlocking', 'This code does not give access.');
            }
            $factory = $user instanceof Closure;
            if (!$factory && $row['claimed_by'] === $user['id']) return [false, $user];
            if ($row['claimed_by'] !== null) {
                throw new ApiError(409, 'already_claimed', 'This code is already in use. If it\'s yours, sign in.');
            }
            // Claimed by the old email-only form: only the account with that email may take it over
            $legacyEmail = strtolower(trim((string) $row['email']));
            if ($legacyEmail !== '' && ($factory || $user['email'] === null || $legacyEmail !== $user['email'])) {
                throw new ApiError(409, 'email_required', 'Sign in with the email this code was registered with.');
            }

            $grants = self::resolveItems($row, $itemId, $variant);
            if ($factory) $user = $user();

            $bound = Db::run('UPDATE qr SET claimed_by = ?, email = ?, claimed_at = ? WHERE code = ? AND claimed_by IS NULL',
                [$user['id'], (string) ($user['email'] ?? ''), qrNow(), $code]);
            if ($bound->rowCount() !== 1) throw new ApiError(409, 'already_claimed', 'This code is already in use. If it\'s yours, sign in.');

            $editionNumber = $row['edition_number'] !== null ? (int) $row['edition_number'] : null;
            foreach ($grants as $grant) {
                Inventory::add($user['id'], $grant['item_id'], $grant['variant'], $grant['qty'], 'qr', $code, $editionNumber);
            }
            return [true, $user];
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
