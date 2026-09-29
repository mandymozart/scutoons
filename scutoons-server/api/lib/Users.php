<?php
declare(strict_types=1);

/**
 * Reader accounts (email only). A new account needs a signup code (permissions.json "signup": true – a SCUTOONS
 * product or a promo code); Auth::verify checks that before calling create().
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
     * Creates the user (after a confirmed login) with the `member` role and the starter items. Call inside a
     * transaction. $signupCode = the code the account was created with (stored in users.signup_code).
     * @return array<string, mixed>
     */
    public static function create(string $email, string $locale, ?string $signupCode = null, bool $marketingOptIn = false): array
    {
        $id = Db::uuid();
        $now = Db::now();
        Db::run('INSERT INTO users (id, email, locale, created_at, confirmed_at, updated_at, signup_code, marketing_opt_in, marketing_opt_in_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)',
            [$id, $email, $locale, $now, $now, $now, $signupCode, $marketingOptIn ? 1 : 0, $marketingOptIn ? $now : null]);
        Roles::grant($id, Roles::MEMBER, 'signup');
        foreach (Catalog::starters() as $item) {
            Inventory::add($id, $item['id'], null, 1, 'starter');
        }
        return self::byId($id);
    }

    /** Sets "Send me news about new releases"; marketing_opt_in_at = time of the last change */
    public static function setMarketingOptIn(string $userId, bool $optIn): void
    {
        $now = Db::now();
        Db::run('UPDATE users SET marketing_opt_in = ?, marketing_opt_in_at = ?, updated_at = ? WHERE id = ? AND marketing_opt_in <> ?',
            [$optIn ? 1 : 0, $now, $now, $userId, $optIn ? 1 : 0]);
    }

    /** PUT /me/ { marketing_opt_in: bool } → { status, user } */
    public static function updateMe(array $body): never
    {
        $user = Auth::user();
        if (!array_key_exists('marketing_opt_in', $body) || !is_bool($body['marketing_opt_in'])) {
            throw new ApiError(400, 'invalid_body', 'marketing_opt_in (true/false) is required.');
        }
        self::setMarketingOptIn($user['id'], $body['marketing_opt_in']);
        Http::json(200, ['status' => 'success', 'user' => self::toJson(self::byId($user['id']))]);
    }

    /** A boolean from a JSON body (true, 1, "1", "true", "on"); anything else = false */
    public static function flag(mixed $value): bool
    {
        return $value === true || $value === 1 || (is_string($value) && in_array(strtolower($value), ['1', 'true', 'on', 'yes'], true));
    }

    /**
     * For the admin "Users" view (generate/users.php) – also the signup code and the opt-in time
     * @param array<string, mixed> $row
     */
    public static function adminJson(array $row): array
    {
        return [
            'id' => $row['id'],
            'email' => $row['email'],
            'created_at' => Db::iso($row['created_at']),
            'roles' => Roles::of((string) $row['id']),
            'signup_code' => $row['signup_code'] ?? null,
            'marketing_opt_in' => (bool) (int) ($row['marketing_opt_in'] ?? 0),
            'marketing_opt_in_at' => Db::iso($row['marketing_opt_in_at'] ?? null),
        ];
    }

    /** @param array<string, mixed> $row */
    public static function toJson(array $row): array
    {
        return [
            'id' => $row['id'],
            'email' => $row['email'],
            'created_at' => Db::iso($row['created_at']),
            'roles' => Roles::of((string) $row['id']),
            'marketing_opt_in' => (bool) (int) ($row['marketing_opt_in'] ?? 0),
        ];
    }
}
