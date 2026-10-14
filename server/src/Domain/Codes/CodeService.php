<?php
declare(strict_types=1);

namespace Scutoons\Domain\Codes;

use Scutoons\Domain\Access\AccessTokens;
use Scutoons\Domain\Access\Permissions;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Catalog\Catalog;
use Scutoons\Domain\Events\EventLog;
use Scutoons\Domain\Inventory\InventoryService;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Time;
use PDOException;

/**
 * QR codes (table `qr`) → items (agents/ITEMS.md "Codes → items"): a code contains its `code_items` rows; without rows
 * but with an edition, the edition's catalog items are the options (the claimer picks item + variant). One code, one
 * owner (claimed_by); viewing with a code is device-level and needs no owner (access()).
 */
final class CodeService
{
    public function __construct(
        private readonly Database $db,
        private readonly Clock $clock,
        private readonly Catalog $catalog,
        private readonly Permissions $permissions,
        private readonly AccessTokens $tokens,
        private readonly EventLog $events,
        private readonly InventoryService $inventory,
    ) {
    }

    /** @return array<string, mixed>|null */
    public function find(string $code): ?array
    {
        return $this->db->one('SELECT * FROM qr WHERE code = ?', [$code]);
    }

    /** @return list<array{item_id: string, variant: ?string, qty: int}> the fixed contents (code_items) */
    public function items(string $code): array
    {
        $rows = $this->db->all('SELECT item_id, variant, qty FROM code_items WHERE code = ? ORDER BY id', [$code]);
        return array_map(fn(array $r) => ['item_id' => (string) $r['item_id'], 'variant' => $r['variant'] !== null ? (string) $r['variant'] : null, 'qty' => (int) $r['qty']], $rows);
    }

    /**
     * The items to choose from (only when the code has no code_items but an edition)
     * @param array<string, mixed> $row
     * @param list<array{item_id: string, variant: ?string, qty: int}>|null $fixedItems
     * @return list<array{item_id: string, variants: list<string>}>
     */
    public function options(array $row, ?array $fixedItems = null): array
    {
        $fixedItems ??= $this->items((string) $row['code']);
        if ($fixedItems || empty($row['edition_id'])) return [];
        return array_map(fn(array $item) => ['item_id' => (string) $item['id'], 'variants' => $this->catalog->variantIds($item)],
            $this->catalog->itemsForEdition((string) $row['edition_id']));
    }

    /**
     * @param array<string, mixed> $row
     * @return array{id: string, name: string, number: ?int, count: ?int}|null
     */
    public function edition(array $row): ?array
    {
        if (empty($row['edition_id'])) return null;
        $edition = $this->catalog->edition((string) $row['edition_id']);
        return [
            'id' => (string) $row['edition_id'],
            'name' => (string) ($edition['name'] ?? $row['edition_id']),
            'number' => $row['edition_number'] !== null ? (int) $row['edition_number'] : null,
            'count' => isset($edition['edition_count']) ? (int) $edition['edition_count'] : null,
        ];
    }

    /**
     * A code the old system registered to $email (the `email` column, not yet in an account) that can create its account –
     * the address is the proof, so its owner signs up without typing a code (owner 2026-10-03). The oldest one; the
     * others stay registered to the address and are added by scanning them (signupRefusal / claim accept the match).
     */
    public function legacySignupCode(string $email): ?string
    {
        $rows = $this->db->all('SELECT code FROM qr WHERE claimed_by IS NULL AND LOWER(TRIM(email)) = ? ORDER BY created_at, code LIMIT 20', [strtolower(trim($email))]);
        foreach ($rows as $row) {
            if ($this->signupRefusal((string) $row['code'], $email) === null) return (string) $row['code'];
        }
        return null;
    }

    /**
     * Can this code create a new account for $email? (ITEMS.md "Accounts only with a product or a PR code")
     * Existing, not expired, not revoked, type with `"signup": true`; a claimable code (a product) additionally must
     * still be free (or registered to this email by the old form) – it is claimed at signup.
     * @return string|null null = yes; else the reason: not_found | expired | revoked | no_signup | already_claimed
     */
    public function signupRefusal(?string $code, string $email): ?string
    {
        if ($code === null) return 'not_found';
        $row = $this->find($code);
        if (!$row) return 'not_found';
        if (CodeRules::isExpired($row, $this->clock->time())) return 'expired';
        if (CodeRules::isRevoked($row)) return 'revoked';
        $type = (string) $row['type'];
        if (!$this->permissions->signup($type)) return 'no_signup';
        if ($this->permissions->claimable($type)) {
            $legacyEmail = strtolower(trim((string) $row['email']));
            if ($row['claimed_by'] !== null || ($legacyEmail !== '' && $legacyEmail !== $email)) return 'already_claimed';
        }
        if ($this->inviteCodeUsed($code)) return 'used';
        return null;
    }

    /**
     * A promo code mailed with a gift invite (Domain\Campaigns\Invites) makes one account: only while its invite waits
     * (read here, not through Invites – that one depends on this class). Nothing before the 2026-10-14 migration.
     */
    private function inviteCodeUsed(string $code): bool
    {
        try {
            $status = $this->db->value('SELECT status FROM campaign_invites WHERE invite_code = ? LIMIT 1', [$code]);
        } catch (PDOException $error) {
            if ($error->getCode() === '42S02' || preg_match('/Base table or view not found/i', $error->getMessage()) === 1) return false;
            throw $error;
        }
        return $status !== null && $status !== 'waiting';
    }

    /**
     * An existing, unexpired, unrevoked code or 404 not_found / 410 expired / 410 revoked
     * @return array<string, mixed>
     */
    public function valid(string $code): array
    {
        $row = $this->find($code) ?? throw new ApiError(404, 'not_found', 'This code does not exist.');
        $this->assertValid($row);
        return $row;
    }

    /** @param array<string, mixed> $row */
    private function assertValid(array $row): void
    {
        if (CodeRules::isExpired($row, $this->clock->time())) throw new ApiError(410, 'expired', 'This code has expired.');
        if (CodeRules::isRevoked($row)) throw new ApiError(410, 'revoked', 'This code is no longer valid.');
    }

    /**
     * GET|POST /access/ → device access for anyone holding the code (also when someone else owns it; never says who):
     * { status, code, type, expires_at, permissions, claimable, signup, claimed, claimed_by_me?, access_token? }
     * access_token only when the code grants "catalog". Logs an `access` event (once per device and day).
     * @param array<string, mixed>|null $user the signed-in user, if any
     * @return array<string, mixed>
     */
    public function access(string $code, ?array $user, ?string $toon): array
    {
        $row = $this->valid($code);
        $type = $this->permissions->forType((string) $row['type']);
        $expires = $row['expires_at'] ? (int) strtotime((string) $row['expires_at']) : null;
        $response = [
            'status' => 'success',
            'code' => $row['code'],
            'type' => $row['type'],
            'expires_at' => $expires !== null ? date('c', $expires) : null,
            'permissions' => $type['grants'],
            'claimable' => $type['claimable'],
            'signup' => $type['signup'],
            'claimed' => CodeRules::isClaimed($row),
        ];
        if ($user !== null) $response['claimed_by_me'] = $row['claimed_by'] === $user['id'];
        if (in_array('catalog', $type['grants'], true)) {
            $response['access_token'] = $this->tokens->issue($type['grants'], $expires, $this->tokens->codeSubject((string) $row['code']));
        }
        $this->events->logAccess((string) $row['code'], isset($user['id']) ? (string) $user['id'] : null, $toon);
        return $response;
    }

    /** A scan of the QR landing page (qr app): logged for every existing code, also expired / revoked ones */
    public function logScan(string $code): void
    {
        $row = $this->find($code);
        if ($row) $this->events->log('scan', (string) $row['code'], null, EventLog::toonFromUrl((string) $row['redirect_to']));
    }

    /**
     * GET /code/?c= → public info about a code (never an email)
     * @param array<string, mixed>|null $user
     * @return array<string, mixed>
     */
    public function info(string $code, ?array $user): array
    {
        $row = $this->valid($code);
        $items = $this->items($code);
        $type = $this->permissions->forType((string) $row['type']);
        $response = [
            'code' => $row['code'],
            'type' => $row['type'],
            'claimable' => $type['claimable'],
            'signup' => $type['signup'],
            'claimed' => CodeRules::isClaimed($row),
        ];
        if ($user !== null) $response['claimed_by_me'] = $row['claimed_by'] === $user['id'];
        $response += [
            'items' => $items,
            'options' => $this->options($row, $items),
            'edition' => $this->edition($row),
            'redirect_to' => $row['redirect_to'],
            'expires_at' => Time::localIso($row['expires_at']),
        ];
        return $response;
    }

    /**
     * Claims a code for a user: adds its items (or the chosen option) to the inventory and binds the code.
     * Same user again → no change (idempotent). Returns true when the code was newly claimed.
     * Throws 404 not_found, 410 expired/revoked, 422 not_claimable, 409 already_claimed/email_required,
     * 422 choose_item / invalid_item. Logs claim / claim_refused.
     * @param array<string, mixed> $user
     */
    public function claim(array $user, string $code, ?string $itemId, ?string $variant): bool
    {
        $code = CodeRules::normalize($code) ?? throw new ApiError(400, 'invalid_code', 'Code missing or malformed.');
        try {
            $new = $this->db->transaction(fn() => $this->claimInTransaction($user, $code, $itemId, $variant));
        } catch (ApiError $error) {
            // Statistics: refusals of an existing code (not typos, not the item choice)
            if (in_array($error->errorCode, ['already_claimed', 'email_required', 'not_claimable', 'revoked', 'expired'], true)) {
                $this->events->log('claim_refused', $code, (string) $user['id']);
            }
            throw $error;
        }
        if ($new) $this->events->log('claim', $code, (string) $user['id']);
        return $new;
    }

    /**
     * The claim itself – call inside Database::transaction() (the login service uses it to create an account and claim
     * its product code atomically). Locks the qr row (FOR UPDATE).
     * No events are logged here. Returns true when newly claimed.
     * @param array<string, mixed> $user
     */
    public function claimInTransaction(array $user, string $code, ?string $itemId, ?string $variant): bool
    {
        $row = $this->db->one('SELECT * FROM qr WHERE code = ? FOR UPDATE', [$code]);
        if (!$row) throw new ApiError(404, 'not_found', 'This code does not exist.');
        $this->assertValid($row);
        if ($row['claimed_by'] === $user['id']) return false;
        // Which types can be owned: the permissions map
        if (!$this->permissions->claimable((string) $row['type'])) {
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

        $grants = $this->resolveItems($row, $itemId, $variant);

        $bound = $this->db->run('UPDATE qr SET claimed_by = ?, email = ?, claimed_at = ? WHERE code = ? AND claimed_by IS NULL',
            [$user['id'], $user['email'], $this->clock->local(), $code]);
        if ($bound->rowCount() !== 1) throw new ApiError(409, 'already_claimed', 'This code is already in someone else\'s inventory.');

        $editionNumber = $row['edition_number'] !== null ? (int) $row['edition_number'] : null;
        foreach ($grants as $grant) {
            $this->inventory->add((string) $user['id'], $grant['item_id'], $grant['variant'], $grant['qty'], 'qr', $code, $editionNumber);
        }
        return true;
    }

    /**
     * What a claim adds: the code_items, or the one option chosen (auto-picked when there is only one)
     * @param array<string, mixed> $row
     * @return list<array{item_id: string, variant: ?string, qty: int}>
     */
    private function resolveItems(array $row, ?string $itemId, ?string $variant): array
    {
        $fixed = $this->items((string) $row['code']);
        if ($fixed) {
            return array_values(array_filter($fixed, function (array $grant) use ($row) {
                if ($this->catalog->item($grant['item_id'])) return true;
                error_log("[qr-api] code {$row['code']}: unknown item {$grant['item_id']} in code_items – skipped");
                return false;
            }));
        }
        $options = $this->options($row, $fixed);
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
