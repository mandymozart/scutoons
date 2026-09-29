<?php
declare(strict_types=1);

/**
 * QR codes → items (ITEMS.md "Codes → items"): a code contains its `code_items` rows; without rows but with an
 * edition, the edition's catalog items are the options (the claimer picks item + variant). One code, one user.
 */
final class Codes
{
    /** A syntactically valid code (generated codes are nanoid(8); up to 32 chars allowed for hand-made test codes) or null */
    public static function normalize(mixed $value): ?string
    {
        if (!is_string($value)) return null;
        $code = trim($value);
        return preg_match('/^[A-Za-z0-9_-]{1,32}$/', $code) ? $code : null;
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

    /**
     * Can this code create a new account for $email? (ITEMS.md "Accounts only with a product or a PR code")
     * Existing, not expired, not revoked, type with `"signup": true` in permissions.json; a claimable code (a
     * product) additionally must still be free (or registered to this email by the old form) – it is claimed at signup.
     * @return string|null null = yes; else the reason: not_found | expired | revoked | no_signup | already_claimed
     */
    public static function signupRefusal(?string $code, string $email): ?string
    {
        if ($code === null) return 'not_found';
        $row = self::find($code);
        if (!$row) return 'not_found';
        if (qrIsExpired($row)) return 'expired';
        if (qrIsRevoked($row)) return 'revoked';
        $type = (string) $row['type'];
        if (!Permissions::signup($type)) return 'no_signup';
        if (Permissions::claimable($type)) {
            $legacyEmail = strtolower(trim((string) $row['email']));
            if ($row['claimed_by'] !== null || ($legacyEmail !== '' && $legacyEmail !== $email)) return 'already_claimed';
        }
        return null;
    }

    public static function isClaimed(array $row): bool
    {
        return $row['claimed_by'] !== null || (string) $row['email'] !== '';
    }

    /** Per-IP limit for public code lookups (/access/, /code/) – every attempt counts, against code enumeration */
    private static function guardLookup(): void
    {
        $bucket = 'lookup:' . hash_hmac('sha256', Http::clientIp(), Auth::secret());
        RateLimit::guard($bucket, Config::int('ACCESS_MAX_PER_IP_HOUR'), 3600, 'Too many codes checked – try again later.');
        RateLimit::hit($bucket);
        RateLimit::prune();
    }

    /**
     * GET|POST /access/ { code } or ?c= (optional Bearer) → device access for anyone holding the code:
     * { status, code, type, expires_at, permissions, claimable, signup, claimed, claimed_by_me?, access_token? }
     * Works for codes owned by someone else too (viewing is device-level); never says who owns it.
     * access_token only when the code grants "catalog".
     */
    public static function access(): never
    {
        self::guardLookup();
        $input = ($_SERVER['REQUEST_METHOD'] ?? 'GET') === 'POST' ? Http::body() : [];
        $code = self::normalize($input['code'] ?? $input['c'] ?? $_GET['c'] ?? $_GET['code'] ?? null)
            ?? throw new ApiError(400, 'invalid_code', 'Code missing or malformed.');
        $row = self::find($code) ?? throw new ApiError(404, 'not_found', 'This code does not exist.');
        if (qrIsExpired($row)) throw new ApiError(410, 'expired', 'This code has expired.');
        if (qrIsRevoked($row)) throw new ApiError(410, 'revoked', 'This code is no longer valid.');

        $type = Permissions::forType((string) $row['type']);
        $expires = $row['expires_at'] ? (int) strtotime((string) $row['expires_at']) : null;
        $response = [
            'status' => 'success',
            'code' => $row['code'],
            'type' => $row['type'],
            'expires_at' => $expires !== null ? date('c', $expires) : null,
            'permissions' => $type['grants'],
            'claimable' => $type['claimable'],
            'signup' => $type['signup'],
            'claimed' => self::isClaimed($row),
        ];
        $user = Auth::optionalUser();
        if ($user !== null) $response['claimed_by_me'] = $row['claimed_by'] === $user['id'];
        if (in_array('catalog', $type['grants'], true)) {
            $response['access_token'] = Permissions::issueToken($type['grants'], $expires, Permissions::codeSubject($row['code']));
        }
        $toon = $input['toon'] ?? $_GET['toon'] ?? null;
        Events::logAccess($row['code'], $user['id'] ?? null, is_string($toon) ? $toon : null);
        Http::json(200, $response);
    }

    /** GET /code/?c=CODE → public info about a code (never an email) */
    public static function get(): never
    {
        self::guardLookup();
        $code = self::normalize($_GET['c'] ?? $_GET['code'] ?? null);
        if ($code === null) throw new ApiError(400, 'invalid_code', 'Code missing or malformed.');
        $row = self::find($code);
        if (!$row) throw new ApiError(404, 'not_found', 'This code does not exist.');
        if (qrIsExpired($row)) throw new ApiError(410, 'expired', 'This code has expired.');
        if (qrIsRevoked($row)) throw new ApiError(410, 'revoked', 'This code is no longer valid.');

        $items = self::items($code);
        $type = Permissions::forType((string) $row['type']);
        $response = [
            'code' => $row['code'],
            'type' => $row['type'],
            'claimable' => $type['claimable'],
            'signup' => $type['signup'],
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
        try {
            $new = self::claimLocked($user, $code, $itemId, $variant);
        } catch (ApiError $error) {
            // Statistics: refusals of an existing code (not typos, not the item choice)
            if (in_array($error->errorCode, ['already_claimed', 'email_required', 'not_claimable', 'revoked', 'expired'], true)) {
                Events::log('claim_refused', $code, $user['id']);
            }
            throw $error;
        }
        if ($new) Events::log('claim', $code, $user['id']);
        return $new;
    }

    private static function claimLocked(array $user, string $code, ?string $itemId, ?string $variant): bool
    {
        return Db::transaction(fn() => self::claimInTransaction($user, $code, $itemId, $variant));
    }

    /**
     * The claim itself – call inside Db::transaction() (Auth::confirm uses it to create an account and claim its
     * product code atomically). Locks the qr row (MySQL FOR UPDATE; SQLite: the transaction's write lock).
     * No events are logged here. Returns true when newly claimed.
     */
    public static function claimInTransaction(array $user, string $code, ?string $itemId, ?string $variant): bool
    {
        $row = Db::one('SELECT * FROM qr WHERE code = ?' . Db::forUpdate(), [$code]);
        if (!$row) throw new ApiError(404, 'not_found', 'This code does not exist.');
        if (qrIsExpired($row)) throw new ApiError(410, 'expired', 'This code has expired.');
        if (qrIsRevoked($row)) throw new ApiError(410, 'revoked', 'This code is no longer valid.');
        if ($row['claimed_by'] === $user['id']) return false;
        // Which types can be owned: the permissions map (api/permissions.json)
        if (!Permissions::claimable((string) $row['type'])) {
            // e.g. promo codes: they unlock reading and allow a signup, but nobody owns them
            throw new ApiError(422, 'not_claimable', 'This code cannot be added to an inventory.');
        }
        if ($row['claimed_by'] !== null) {
            throw new ApiError(409, 'already_claimed', 'This code is already in someone else\'s account. You can still read with it.');
        }
        // Claimed by the old email-only form: only the account with that email may own it
        $legacyEmail = strtolower(trim((string) $row['email']));
        if ($legacyEmail !== '' && $legacyEmail !== $user['email']) {
            throw new ApiError(409, 'email_required', 'Sign in with the email this code was registered with.');
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
