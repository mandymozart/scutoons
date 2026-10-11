<?php
declare(strict_types=1);

namespace Scutoons\Domain\Roles;

use InvalidArgumentException;
use Scutoons\Domain\ApiError;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Database\Database;

/**
 * Account roles (agents/ITEMS.md "Roles, signup rule, promo codes"). Tables `roles` (seeded 1 member, 2 admin,
 * 3 creator) and `user_roles`. Every account gets `member` at signup; `creator` may use the creator app and the content
 * API; `admin` may use the admin app and /api/admin/*; `developer` (above admin, BACKLOG 10.61) runs the database
 * migrations (/api/dev/*) – only a developer grants or revokes it, and the last one stays.
 * HTTP: the RequireRole middleware (or require() inside a controller) → 401 unauthorized / 403 forbidden { role }.
 * Outside HTTP: `bin/console roles grant|revoke|list`, admin API /api/admin/users/{id}/roles.
 */
final class Roles
{
    public const MEMBER = 'member';
    public const ADMIN = 'admin';
    public const CREATOR = 'creator';
    public const DEVELOPER = 'developer';

    public function __construct(private readonly Database $db, private readonly Clock $clock)
    {
    }

    /** @return list<array{id: int, name: string, description: string}> every known role */
    public function all(): array
    {
        return array_map(fn(array $r) => ['id' => (int) $r['id'], 'name' => (string) $r['name'], 'description' => (string) $r['description']],
            $this->db->all('SELECT id, name, description FROM roles ORDER BY id'));
    }

    public function exists(string $role): bool
    {
        return $this->id($role) !== null;
    }

    /** @return list<string> the role names of a user (member first, then by id) */
    public function of(string $userId): array
    {
        return array_map(fn(array $r) => (string) $r['name'],
            $this->db->all('SELECT r.name FROM user_roles ur JOIN roles r ON r.id = ur.role_id WHERE ur.user_id = ? ORDER BY r.id', [$userId]));
    }

    /** @param array<string, mixed>|string $user a users row or a user id */
    public function has(array|string $user, string $role): bool
    {
        $userId = is_array($user) ? (string) ($user['id'] ?? '') : $user;
        if ($userId === '') return false;
        return $this->db->value('SELECT 1 FROM user_roles ur JOIN roles r ON r.id = ur.role_id WHERE ur.user_id = ? AND r.name = ?', [$userId, $role]) !== null;
    }

    /**
     * The signed-in user when they have the role: null user → 401 unauthorized; without the role → 403 forbidden.
     * @param array<string, mixed>|null $user
     * @return array<string, mixed>
     */
    public function require(?array $user, string $role): array
    {
        if ($user === null) throw new ApiError(401, 'unauthorized', 'Not signed in.');
        if (!$this->has($user, $role)) throw new ApiError(403, 'forbidden', "This needs the $role role.", ['role' => $role]);
        return $user;
    }

    /**
     * Who may edit content – the creator app: /api/entities/* (read, save, versions, publish, unpublish, restore) and the
     * library's ?draft=1. FOR NOW (owner 2026-10-01) `admin` counts too; to narrow it later, change only this list (and
     * `APP_ROLES.creator` in shared/auth). docs/API.md "Roles".
     */
    public const CONTENT_EDITORS = [self::CREATOR, self::ADMIN];

    /** @param array<string, mixed>|string $user a users row or a user id */
    public function canEditContent(array|string $user): bool
    {
        foreach (self::CONTENT_EDITORS as $role) {
            if ($this->has($user, $role)) return true;
        }
        return false;
    }

    /**
     * The signed-in user when they may edit content: null → 401 unauthorized; otherwise → 403 forbidden { role: "creator" }
     * @param array<string, mixed>|null $user
     * @return array<string, mixed>
     */
    public function requireContentEditor(?array $user): array
    {
        if ($user === null) throw new ApiError(401, 'unauthorized', 'Not signed in.');
        if (!$this->canEditContent($user)) throw new ApiError(403, 'forbidden', 'This needs the creator role.', ['role' => self::CREATOR]);
        return $user;
    }

    /** Gives a user a role; false when they had it already. Throws InvalidArgumentException for unknown roles. */
    public function grant(string $userId, string $role, ?string $grantedBy = null): bool
    {
        $roleId = $this->id($role) ?? throw new InvalidArgumentException("Unknown role $role");
        if ($this->has($userId, $role)) return false;
        $this->db->run('INSERT INTO user_roles (user_id, role_id, granted_at, granted_by) VALUES (?, ?, ?, ?)',
            [$userId, $roleId, $this->clock->utc(), $grantedBy !== null ? substr($grantedBy, 0, 64) : null]);
        return true;
    }

    /** Takes a role away; false when the user did not have it. */
    public function revoke(string $userId, string $role): bool
    {
        $roleId = $this->id($role) ?? throw new InvalidArgumentException("Unknown role $role");
        return $this->db->run('DELETE FROM user_roles WHERE user_id = ? AND role_id = ?', [$userId, $roleId])->rowCount() > 0;
    }

    /** How many accounts have the role */
    public function count(string $role): int
    {
        return (int) $this->db->value('SELECT COUNT(*) FROM user_roles ur JOIN roles r ON r.id = ur.role_id WHERE r.name = ?', [$role]);
    }

    private function id(string $role): ?int
    {
        $id = $this->db->value('SELECT id FROM roles WHERE name = ?', [$role]);
        return $id === null ? null : (int) $id;
    }
}
