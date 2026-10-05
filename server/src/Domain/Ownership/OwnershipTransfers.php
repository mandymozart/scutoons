<?php
declare(strict_types=1);

namespace Scutoons\Domain\Ownership;

use Scutoons\Domain\Access\Permissions;
use Scutoons\Domain\Accounts\Users;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Catalog\Catalog;
use Scutoons\Domain\Codes\CodeService;
use Scutoons\Domain\Mail\MailFailed;
use Scutoons\Domain\Mail\Mailer;
use Scutoons\Domain\Mail\MailTemplates;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Time;
use PDOException;

/**
 * Ownership changes (agents/ITEMS.md "Ownership, claims, gifting"): THE transfer core that moves a QR code – and every
 * inventory entry bound to it – from one account to another, the transfer history (table item_transfers), the direct
 * admin transfer, the "you received …" notices and the two ownership mails.
 *
 * One code = one owner stays true at every moment: the DB guard `qr_owner_immutable` forbids A → B in one UPDATE, so the
 * core goes A → NULL → B inside one transaction (the guard allows both steps), then moves the inventory rows (the guard
 * `inventory_code_owner_update` checks they follow the code's owner) and drops the previous owner's avatar slots that
 * wore them. The previous owner keeps reading with the code on their devices (device access is not ownership).
 */
final class OwnershipTransfers
{
    public const KINDS = ['claim', 'gift', 'admin'];

    public function __construct(
        private readonly Database $db,
        private readonly Clock $clock,
        private readonly Users $users,
        private readonly CodeService $codes,
        private readonly Permissions $permissions,
        private readonly Catalog $catalog,
        private readonly Mailer $mailer,
        private readonly Settings $settings,
    ) {
    }

    /**
     * Moves a code to $to – call inside Database::transaction(). Registered code: its entries move (avatar slots of the
     * previous owner that wore them are cleared). Unregistered (or registered by the old email-only form, no account):
     * the code is claimed for $to (its items are added; a code with several options needs $itemId / $variant, else 422
     * choose_item). Revoked / expired codes can move when registered (an admin decides); claiming a free one follows the
     * normal claim rules. Records the transfer.
     * 404 not_found · 422 not_claimable · 409 already_owner.
     * @param array<string, mixed> $to the new owner (users row)
     * @return array{code: string, from: ?string, to: string, entries: list<string>, item_id: ?string, variant: ?string, edition_number: ?int, transfer_id: int}
     */
    public function moveCode(string $code, array $to, string $kind, ?string $byUserId, ?string $note, ?string $claimId = null, ?string $itemId = null, ?string $variant = null): array
    {
        $row = $this->db->one('SELECT * FROM qr WHERE code = ? FOR UPDATE', [$code]) ?? throw new ApiError(404, 'not_found', 'This code does not exist.');
        if (!$this->permissions->claimable((string) $row['type'])) throw new ApiError(422, 'not_claimable', 'Nobody can own this code.');
        $toId = (string) $to['id'];
        $from = $row['claimed_by'] !== null ? (string) $row['claimed_by'] : null;
        if ($from === $toId) throw new ApiError(409, 'already_owner', 'This code is already in that account.');
        $now = $this->clock->utc();

        if ($from !== null) {
            $entries = array_map(fn(array $r) => (string) $r['id'], $this->db->all('SELECT id FROM inventory WHERE code = ? ORDER BY acquired_at, id', [$code]));
            // A → NULL → B: the owner guard never sees two owners
            $this->db->run('UPDATE qr SET claimed_by = NULL WHERE code = ?', [$code]);
            $this->db->run('UPDATE qr SET claimed_by = ?, email = ?, claimed_at = ? WHERE code = ? AND claimed_by IS NULL', [$toId, $to['email'], $this->clock->local(), $code]);
            if ($entries) {
                $marks = implode(',', array_fill(0, count($entries), '?'));
                $this->db->run("DELETE FROM avatar_slots WHERE user_id = ? AND entry_id IN ($marks)", [$from, ...$entries]);
                $this->db->run('UPDATE inventory SET user_id = ?, source = ?, acquired_at = ?, updated_at = ? WHERE code = ?',
                    [$toId, $kind === 'gift' ? 'gift' : 'qr', $now, $now, $code]);
            }
        } else {
            // never registered to an account: the old email-only registration (if any) is replaced, the items added
            $this->db->run("UPDATE qr SET email = '' WHERE code = ? AND claimed_by IS NULL", [$code]);
            $this->codes->claimInTransaction($to, $code, $itemId, $variant);
            $entries = array_map(fn(array $r) => (string) $r['id'], $this->db->all('SELECT id FROM inventory WHERE code = ? ORDER BY acquired_at, id', [$code]));
            if ($kind === 'gift' && $entries) $this->db->run("UPDATE inventory SET source = 'gift' WHERE code = ?", [$code]);
        }

        $first = $entries ? $this->db->one('SELECT item_id, variant FROM inventory WHERE id = ?', [$entries[0]]) : null;
        $itemId = $first !== null ? (string) $first['item_id'] : null;
        $variant = $first !== null && $first['variant'] !== null ? (string) $first['variant'] : null;
        $id = $this->record($kind, $code, $itemId, $variant, max(1, count($entries)), $from, $toId, $byUserId, $claimId, $note);
        return ['code' => $code, 'from' => $from, 'to' => $toId, 'entries' => $entries, 'item_id' => $itemId, 'variant' => $variant,
            'edition_number' => $row['edition_number'] !== null ? (int) $row['edition_number'] : null, 'transfer_id' => $id];
    }

    /** Appends one row to the transfer history (inside the caller's transaction) and returns its id */
    public function record(string $kind, ?string $code, ?string $itemId, ?string $variant, int $qty, ?string $from, string $to, ?string $by, ?string $claimId, ?string $note): int
    {
        $this->db->run('INSERT INTO item_transfers (kind, code, item_id, variant, qty, from_user_id, to_user_id, by_user_id, claim_id, note, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)',
            [$kind, $code, $itemId, $variant, $qty, $from, $to, $by, $claimId, $note !== null && $note !== '' ? mb_substr($note, 0, 2000) : null, $this->clock->utc()]);
        return $this->db->lastInsertId();
    }

    /**
     * POST /admin/codes/{code}/transfer { to: email | user id, note, item_id?, variant? } – an admin moves a code outside
     * a claim (support case, a sale the two people tell us about). Note required. Mails the previous and the new owner.
     * 400 note_required / invalid_recipient · 404 not_found / unknown_user · 409 already_owner · 422 not_claimable.
     * @param array<string, mixed> $admin
     * @param array<string, mixed> $body
     * @return array<string, mixed> the moved code (moveCode) + from / to accounts
     */
    public function adminTransfer(array $admin, string $code, array $body): array
    {
        $this->requireTables();
        $note = self::note($body['note'] ?? null) ?? throw new ApiError(400, 'note_required', 'Say why (a note is required).');
        $to = $this->account($body['to'] ?? null);
        $itemId = is_string($body['item_id'] ?? null) && $body['item_id'] !== '' ? $body['item_id'] : null;
        $variant = is_string($body['variant'] ?? null) && $body['variant'] !== '' ? $body['variant'] : null;
        $moved = $this->db->transaction(fn() => $this->moveCode($code, $to, 'admin', (string) $admin['id'], $note, null, $itemId, $variant));
        $this->notifyMoved($moved);
        return $moved + ['from_account' => $this->accountJson($moved['from']), 'to_account' => $this->accountJson($moved['to'])];
    }

    /**
     * A users row by email or id, else 404 unknown_user (400 invalid_recipient for an empty value)
     * @return array<string, mixed>
     */
    public function account(mixed $value): array
    {
        if (!is_string($value) || trim($value) === '') throw new ApiError(400, 'invalid_recipient', 'to: an email address or an account id.');
        $value = trim($value);
        $user = str_contains($value, '@') ? $this->users->byEmail(strtolower($value)) : $this->users->byId($value);
        return $user ?? throw new ApiError(404, 'unknown_user', 'There is no account with this email or id.');
    }

    /** @return array{id: string, email: ?string}|null the account; email null when it was deleted */
    public function accountJson(?string $userId): ?array
    {
        if ($userId === null) return null;
        $user = $this->users->byId($userId);
        return $user ? ['id' => (string) $user['id'], 'email' => (string) $user['email']] : ['id' => $userId, 'email' => null];
    }

    /** A trimmed note (≤ 2000 chars) or null when empty */
    public static function note(mixed $value): ?string
    {
        if (!is_string($value)) return null;
        $note = trim($value);
        return $note === '' ? null : mb_substr($note, 0, 2000);
    }

    /**
     * The ownership history of a code, newest first (admin): { id, kind, item_id, variant, qty, from, to, by, claim_id,
     * note, created_at } with accounts as { id, email }
     * @return list<array<string, mixed>>
     */
    public function history(string $code): array
    {
        try {
            $rows = $this->db->all('SELECT * FROM item_transfers WHERE code = ? ORDER BY created_at DESC, id DESC LIMIT 200', [$code]);
        } catch (PDOException $error) {
            if (!self::missingTable($error)) throw $error;
            return []; // before the migration (the admin code page still loads)
        }
        return array_map(fn(array $r) => $this->transferJson($r), $rows);
    }

    /**
     * @param array<string, mixed> $r an item_transfers row
     * @return array<string, mixed>
     */
    public function transferJson(array $r): array
    {
        return [
            'id' => (int) $r['id'],
            'kind' => $r['kind'],
            'code' => $r['code'],
            'item_id' => $r['item_id'],
            'variant' => $r['variant'],
            'qty' => (int) $r['qty'],
            'from' => $this->accountJson($r['from_user_id'] !== null ? (string) $r['from_user_id'] : null),
            'to' => $this->accountJson((string) $r['to_user_id']),
            'by' => $this->accountJson($r['by_user_id'] !== null ? (string) $r['by_user_id'] : null),
            'claim_id' => $r['claim_id'],
            'note' => $r['note'],
            'created_at' => Time::iso($r['created_at']),
        ];
    }

    /**
     * What arrived in this account and was not seen yet (GET /me/ `received`): gifts (with the giver's email – the
     * giver is told so) and codes an admin moved here (no previous owner shown). Newest first, at most 20.
     * @return list<array{id: int, kind: string, item_id: ?string, variant: ?string, qty: int, code: ?string, from: ?string, created_at: ?string}>
     */
    public function unseen(string $userId): array
    {
        try {
            $rows = $this->unseenRows($userId);
        } catch (PDOException $error) {
            if (!self::missingTable($error)) throw $error;
            return []; // before the 2026-10-02-ownership migration: nothing to show (GET /me/ must keep working)
        }
        return array_map(fn(array $r) => [
            'id' => (int) $r['id'],
            'kind' => (string) $r['kind'],
            'item_id' => $r['item_id'] !== null ? (string) $r['item_id'] : null,
            'variant' => $r['variant'] !== null ? (string) $r['variant'] : null,
            'qty' => (int) $r['qty'],
            'code' => $r['code'] !== null ? (string) $r['code'] : null,
            'from' => $r['kind'] === 'gift' && $r['from_email'] !== null ? (string) $r['from_email'] : null,
            'created_at' => Time::iso($r['created_at']),
        ], $rows);
    }

    /**
     * A query failed because a table of the 2026-10-02-ownership migration is missing (SQLite "no such table", MySQL
     * 42S02 "Base table or view not found") – production runs the code before the migration is applied in phpMyAdmin
     */
    public static function missingTable(PDOException $error): bool
    {
        return $error->getCode() === '42S02' || preg_match('/no such table|Base table or view not found/i', $error->getMessage()) === 1;
    }

    /**
     * 503 not_available when the tables of the 2026-10-02-ownership migration are missing (write paths: gift, claim,
     * transfer) – a clear answer instead of a 500 until the migration ran
     */
    public function requireTables(): void
    {
        try {
            $this->db->value('SELECT 1 FROM item_transfers LIMIT 1');
            $this->db->value('SELECT 1 FROM ownership_claims LIMIT 1');
        } catch (PDOException $error) {
            if (!self::missingTable($error)) throw $error;
            throw new ApiError(503, 'not_available', 'Gifts and ownership claims are not available yet – try again later.');
        }
    }

    /** @return list<array<string, mixed>> */
    private function unseenRows(string $userId): array
    {
        return $this->db->all('SELECT t.id, t.kind, t.item_id, t.variant, t.qty, t.code, t.created_at, u.email AS from_email FROM item_transfers t
            LEFT JOIN users u ON u.id = t.from_user_id WHERE t.to_user_id = ? AND t.seen_at IS NULL ORDER BY t.created_at DESC, t.id DESC LIMIT 20', [$userId]);
    }

    /**
     * POST /inventory/received/seen { ids? } – marks the notices as seen (all when no ids). Returns how many changed.
     * @param mixed $ids
     */
    public function markSeen(string $userId, mixed $ids): int
    {
        $now = $this->clock->utc();
        if (is_array($ids) && $ids !== []) {
            $ids = array_values(array_filter(array_map(fn($v) => is_int($v) || (is_string($v) && ctype_digit($v)) ? (int) $v : null, $ids), fn($v) => $v !== null));
            if ($ids === []) return 0;
            $marks = implode(',', array_fill(0, count($ids), '?'));
            return $this->db->run("UPDATE item_transfers SET seen_at = ? WHERE to_user_id = ? AND seen_at IS NULL AND id IN ($marks)", [$now, $userId, ...$ids])->rowCount();
        }
        return $this->db->run('UPDATE item_transfers SET seen_at = ? WHERE to_user_id = ? AND seen_at IS NULL', [$now, $userId])->rowCount();
    }

    /**
     * The two ownership mails after a code moved (claim decided or admin transfer): to the previous owner (if it was an
     * account) and to the new one, each in their language. A failed mail never undoes the transfer (it is logged).
     * @param array{code: string, from: ?string, to: string, item_id: ?string, variant: ?string, edition_number: ?int} $moved
     */
    public function notifyMoved(array $moved): void
    {
        $url = rtrim($this->settings->get('APP_URL'), '/') . '/account';
        foreach ([[$moved['from'], MailTemplates::OWNERSHIP_TRANSFERRED_FROM], [$moved['to'], MailTemplates::OWNERSHIP_TRANSFERRED_TO]] as [$userId, $template]) {
            $user = $userId !== null ? $this->users->byId($userId) : null;
            if (!$user) continue;
            $locale = MailTemplates::locale($user['locale'] ?? null);
            try {
                $this->mailer->send($template, (string) $user['email'], $locale, [
                    'code' => $moved['code'],
                    'item' => $this->itemName($moved['item_id'], $moved['variant'], $locale, $moved['edition_number'], $moved['code']),
                    'url' => $url,
                ]);
            } catch (MailFailed) {
                // logged by the mailer (admin app → Emails); the transfer stands
            }
        }
    }

    /** "Dago Hoodie (Schwarz) #12" – the item's name in a locale (+ variant, + edition number); the code when unknown */
    public function itemName(?string $itemId, ?string $variant, string $locale, ?int $editionNumber = null, ?string $fallback = null): string
    {
        $item = $itemId !== null ? $this->catalog->item($itemId) : null;
        if (!$item) return $fallback ?? (string) $itemId;
        $name = Catalog::text($item['name'] ?? $itemId, $locale);
        $variantDef = $this->catalog->variant($item, $variant);
        if ($variantDef) $name .= ' (' . Catalog::text($variantDef['name'] ?? $variant, $locale) . ')';
        return $editionNumber !== null ? "$name #$editionNumber" : $name;
    }
}
