<?php
declare(strict_types=1);

/**
 * Reader accounts. `email` NULL = anonymous account, created by the first use of an unclaimed code
 * (POST /auth/code/); it can add an email later (auth/request + verify with its Bearer session) and is merged into
 * an existing account with that email.
 */
final class Users
{
    /** @return array<string, mixed>|null */
    public static function byEmail(string $email): ?array
    {
        return Db::one('SELECT * FROM users WHERE email = ?', [$email]);
    }

    /** @return array<string, mixed>|null */
    public static function byId(string $id): ?array
    {
        return Db::one('SELECT * FROM users WHERE id = ?', [$id]);
    }

    /**
     * Creates a user and grants the starter items. `$email` null = anonymous account (not confirmed).
     * Call inside a transaction.
     * @return array<string, mixed>
     */
    public static function create(?string $email, string $locale): array
    {
        $id = Db::uuid();
        $now = Db::now();
        Db::run('INSERT INTO users (id, email, locale, created_at, confirmed_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)',
            [$id, $email, $locale, $now, $email === null ? null : $now, $now]);
        foreach (Catalog::starters() as $item) {
            Inventory::add($id, $item['id'], null, 1, 'starter');
        }
        return self::byId($id);
    }

    /** Gives an anonymous account its (confirmed) email. Call inside a transaction. */
    public static function attachEmail(string $userId, string $email): array
    {
        $now = Db::now();
        Db::run('UPDATE users SET email = ?, confirmed_at = ?, updated_at = ? WHERE id = ? AND email IS NULL', [$email, $now, $now, $userId]);
        Db::run("UPDATE qr SET email = ? WHERE claimed_by = ?", [$email, $userId]);
        return self::byId($userId);
    }

    /**
     * Merges an anonymous account into another account and deletes it. Call inside a transaction.
     * - codes: claimed_by NULL first, then the new owner (the qr_owner_immutable trigger forbids owner → owner)
     * - inventory: rows move; stackables are added to the target's stack; a starter item the target already
     *   got as starter is dropped (no second camera)
     * - avatar: the target keeps its slots, empty slots are filled from the anonymous account
     * - progress: per toon the newer updated_at wins
     */
    public static function merge(string $fromId, string $toId): void
    {
        $to = self::byId($toId) ?? throw new RuntimeException('merge target missing');
        $now = Db::now();

        foreach (Db::all('SELECT code FROM qr WHERE claimed_by = ?', [$fromId]) as $row) {
            Db::run('UPDATE qr SET claimed_by = NULL WHERE code = ?', [$row['code']]);
            Db::run('UPDATE qr SET claimed_by = ?, email = ? WHERE code = ?', [$toId, (string) $to['email'], $row['code']]);
        }

        $targetStarters = array_column(Db::all("SELECT DISTINCT item_id FROM inventory WHERE user_id = ? AND source = 'starter'", [$toId]), 'item_id');
        foreach (Db::all('SELECT * FROM inventory WHERE user_id = ?', [$fromId]) as $row) {
            $item = Catalog::item($row['item_id']);
            $drop = $row['source'] === 'starter' && in_array($row['item_id'], $targetStarters, true);
            if (!$drop && $item && !empty($item['stackable']) && $row['code'] === null) {
                $stack = Db::one('SELECT id FROM inventory WHERE user_id = ? AND item_id = ? AND code IS NULL AND '
                    . ($row['variant'] === null ? 'variant IS NULL' : 'variant = ?') . ' ORDER BY acquired_at LIMIT 1',
                    $row['variant'] === null ? [$toId, $row['item_id']] : [$toId, $row['item_id'], $row['variant']]);
                if ($stack) {
                    Db::run('UPDATE inventory SET qty = qty + ?, updated_at = ? WHERE id = ?', [(int) $row['qty'], $now, $stack['id']]);
                    $drop = true;
                }
            }
            if ($drop) {
                Db::run('DELETE FROM avatar_slots WHERE entry_id = ?', [$row['id']]);
                Db::run('DELETE FROM inventory WHERE id = ?', [$row['id']]);
                continue;
            }
            Db::run('UPDATE inventory SET user_id = ?, updated_at = ? WHERE id = ?', [$toId, $now, $row['id']]);
        }

        foreach (Db::all('SELECT slot, entry_id FROM avatar_slots WHERE user_id = ?', [$fromId]) as $slot) {
            $taken = Db::value('SELECT 1 FROM avatar_slots WHERE user_id = ? AND slot = ?', [$toId, $slot['slot']]) !== null;
            if (!$taken) {
                Db::run('INSERT INTO avatar_slots (user_id, slot, entry_id, updated_at) VALUES (?, ?, ?, ?)', [$toId, $slot['slot'], $slot['entry_id'], $now]);
            }
        }
        Db::run('DELETE FROM avatar_slots WHERE user_id = ?', [$fromId]);

        foreach (Db::all('SELECT toon_id, state, updated_at FROM progress WHERE user_id = ?', [$fromId]) as $p) {
            $current = Db::one('SELECT updated_at FROM progress WHERE user_id = ? AND toon_id = ?', [$toId, $p['toon_id']]);
            if (!$current) {
                Db::run('INSERT INTO progress (user_id, toon_id, state, updated_at) VALUES (?, ?, ?, ?)', [$toId, $p['toon_id'], $p['state'], $p['updated_at']]);
            } elseif ((int) $p['updated_at'] > (int) $current['updated_at']) {
                Db::run('UPDATE progress SET state = ?, updated_at = ? WHERE user_id = ? AND toon_id = ?', [$p['state'], $p['updated_at'], $toId, $p['toon_id']]);
            }
        }
        Db::run('DELETE FROM progress WHERE user_id = ?', [$fromId]);

        Db::run('DELETE FROM sessions WHERE user_id = ?', [$fromId]);
        Db::run('UPDATE login_requests SET anon_user_id = NULL WHERE anon_user_id = ?', [$fromId]);
        Db::run('DELETE FROM users WHERE id = ?', [$fromId]);
    }

    /** @param array<string, mixed> $row */
    public static function toJson(array $row): array
    {
        return [
            'id' => $row['id'],
            'email' => $row['email'],
            'anonymous' => $row['email'] === null,
            'created_at' => Db::iso($row['created_at']),
        ];
    }
}
