<?php
declare(strict_types=1);

/**
 * Account roles (ITEMS.md "Roles, signup rule, promo codes"). Tables `roles` (seeded member / admin / creator) and
 * `user_roles`. Every account gets `member` at signup. `creator` may use scutoons.com/creator and the content API;
 * `admin` is reserved.
 *
 * API for other endpoints (keep it small):
 *   Roles::has($user, 'creator')      bool – $user = a users row (Auth::user()) or a user id
 *   Roles::of($userId)                string[] – role names, sorted (member first)
 *   Roles::requireRole('creator')     the signed-in user, else 401 unauthorized / 403 forbidden
 *   Roles::grant($userId, $role, $by) / Roles::revoke($userId, $role)   true when something changed
 * Grant/revoke from outside: admin SPA "Users" (generate/users.php, generate/user-roles.php) and
 * `php scripts/roles.php grant|revoke|list <email> [role]`.
 */
final class Roles
{
    public const MEMBER = 'member';
    public const ADMIN = 'admin';
    public const CREATOR = 'creator';

    /** @return list<array{id: int, name: string, description: string}> every known role */
    public static function all(): array
    {
        return array_map(fn($r) => ['id' => (int) $r['id'], 'name' => $r['name'], 'description' => $r['description']],
            Db::all('SELECT id, name, description FROM roles ORDER BY id'));
    }

    public static function exists(string $role): bool
    {
        return self::id($role) !== null;
    }

    /** @return string[] the role names of a user (member first, then by id) */
    public static function of(string $userId): array
    {
        return array_map('strval', array_column(
            Db::all('SELECT r.name FROM user_roles ur JOIN roles r ON r.id = ur.role_id WHERE ur.user_id = ? ORDER BY r.id', [$userId]), 'name'));
    }

    /** @param array<string, mixed>|string $user a users row or a user id */
    public static function has(array|string $user, string $role): bool
    {
        $userId = is_array($user) ? (string) ($user['id'] ?? '') : $user;
        if ($userId === '') return false;
        return Db::value('SELECT 1 FROM user_roles ur JOIN roles r ON r.id = ur.role_id WHERE ur.user_id = ? AND r.name = ?', [$userId, $role]) !== null;
    }

    /**
     * For endpoints that need a role: the signed-in user (Bearer), else 401 unauthorized; without the role 403 forbidden.
     * @return array<string, mixed>
     */
    public static function requireRole(string $role): array
    {
        $user = Auth::user();
        if (!self::has($user, $role)) throw new ApiError(403, 'forbidden', "This needs the $role role.", ['role' => $role]);
        return $user;
    }

    /** Gives a user a role; false when they had it already. Throws InvalidArgumentException for unknown roles. */
    public static function grant(string $userId, string $role, ?string $grantedBy = null): bool
    {
        $roleId = self::id($role) ?? throw new InvalidArgumentException("Unknown role $role");
        if (self::has($userId, $role)) return false;
        Db::run('INSERT INTO user_roles (user_id, role_id, granted_at, granted_by) VALUES (?, ?, ?, ?)',
            [$userId, $roleId, Db::now(), $grantedBy !== null ? substr($grantedBy, 0, 64) : null]);
        return true;
    }

    /** Takes a role away; false when the user did not have it. */
    public static function revoke(string $userId, string $role): bool
    {
        $roleId = self::id($role) ?? throw new InvalidArgumentException("Unknown role $role");
        return Db::run('DELETE FROM user_roles WHERE user_id = ? AND role_id = ?', [$userId, $roleId])->rowCount() > 0;
    }

    private static function id(string $role): ?int
    {
        $id = Db::value('SELECT id FROM roles WHERE name = ?', [$role]);
        return $id === null ? null : (int) $id;
    }
}
