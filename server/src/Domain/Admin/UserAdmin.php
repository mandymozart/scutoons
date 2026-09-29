<?php
declare(strict_types=1);

namespace Scutoons\Domain\Admin;

use Scutoons\Domain\Accounts\Users;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Time;

/**
 * Accounts and their roles for the admin (the place that shows emails; code views don't).
 */
final class UserAdmin
{
    public const SORTABLE = ['created_at', 'email', 'codes'];
    public const FILTERS = ['role', 'opt_in'];

    public function __construct(
        private readonly Database $db,
        private readonly Users $users,
        private readonly Roles $roles,
    ) {
    }

    /**
     * Legacy users.php: search by part of an email (newest first), at most $limit
     * @return array{roles: list<array<string, mixed>>, total: int, users: list<array<string, mixed>>}
     */
    public function search(string $q, int $limit): array
    {
        $limit = max(1, min(200, $limit));
        $where = '';
        $params = [];
        if ($q !== '') {
            $where = "WHERE email LIKE ? ESCAPE '!'";
            $params[] = Database::likeContains($q);
        }
        $total = (int) $this->db->value("SELECT COUNT(*) FROM users $where", $params);
        $rows = $this->db->all("SELECT id, email, created_at, signup_code, marketing_opt_in, marketing_opt_in_at FROM users $where ORDER BY created_at DESC, email LIMIT $limit", $params);
        return ['roles' => $this->roles->all(), 'total' => $total, 'users' => array_map([$this->users, 'adminJson'], $rows)];
    }

    /**
     * Admin list: q = part of the email, filters role (name), opt_in (1|0); sort created_at | email | codes (owned).
     * @return array{items: list<mixed>, pagination: array{total: int, page: int, pageSize: int}}
     */
    public function list(ListQuery $query): array
    {
        $conditions = [];
        $params = [];
        if ($query->q !== '') {
            $conditions[] = "u.email LIKE ? ESCAPE '!'";
            $params[] = Database::likeContains(strtolower($query->q));
        }
        if (($role = $query->filter('role')) !== null) {
            $conditions[] = 'EXISTS (SELECT 1 FROM user_roles ur JOIN roles r ON r.id = ur.role_id WHERE ur.user_id = u.id AND r.name = ?)';
            $params[] = $role;
        }
        if (($optIn = $query->filter('opt_in')) !== null) {
            $conditions[] = 'u.marketing_opt_in = ?';
            $params[] = Users::flag($optIn) ? 1 : 0;
        }
        $where = $conditions ? 'WHERE ' . implode(' AND ', $conditions) : '';
        $total = (int) $this->db->value("SELECT COUNT(*) FROM users u $where", $params);
        $column = $query->sortField === 'codes' ? 'codes' : "u.{$query->sortField}";
        $rows = $this->db->all("SELECT u.id, u.email, u.created_at, u.signup_code, u.marketing_opt_in, u.marketing_opt_in_at,
                (SELECT COUNT(*) FROM qr q WHERE q.claimed_by = u.id) AS codes
            FROM users u $where {$query->orderBy($column, 'u.id')} LIMIT {$query->pageSize} OFFSET {$query->offset()}", $params);
        return $query->result(array_map(fn(array $r) => $this->users->adminJson($r) + ['codes' => (int) $r['codes']], $rows), $total);
    }

    /**
     * One account: admin fields + owned codes, inventory size, sessions
     * @return array<string, mixed>
     */
    public function detail(string $id): array
    {
        $user = $this->users->byId($id) ?? throw new ApiError(404, 'not_found', 'User not found.');
        $codes = array_map(fn(array $r) => [
            'code' => $r['code'],
            'type' => $r['type'],
            'edition_id' => $r['edition_id'],
            'edition_number' => $r['edition_number'] !== null ? (int) $r['edition_number'] : null,
            'claimed_at' => $r['claimed_at'],
            'revoked_at' => $r['revoked_at'],
        ], $this->db->all('SELECT code, type, edition_id, edition_number, claimed_at, revoked_at FROM qr WHERE claimed_by = ? ORDER BY claimed_at, code', [$id]));
        $sessions = $this->db->one('SELECT COUNT(*) AS n, MAX(last_used_at) AS last_used_at FROM sessions WHERE user_id = ?', [$id]) ?? [];
        return $this->users->adminJson($user) + [
            'locale' => $user['locale'] ?? null,
            'codes' => $codes,
            'inventory_entries' => (int) $this->db->value('SELECT COUNT(*) FROM inventory WHERE user_id = ? AND qty > 0', [$id]),
            'sessions' => (int) ($sessions['n'] ?? 0),
            'last_seen_at' => Time::iso($sessions['last_used_at'] ?? null),
        ];
    }

    public function idByEmail(string $email): ?string
    {
        $user = $this->users->byEmail($email);
        return $user !== null ? (string) $user['id'] : null;
    }

    /**
     * Grant or revoke a role. 400 unknown_role, 404 not_found.
     * @return array{changed: bool, user: array<string, mixed>}
     */
    public function setRole(string $userId, string $role, bool $grant, string $grantedBy): array
    {
        if (!$this->roles->exists($role)) throw new ApiError(400, 'unknown_role', "Unknown role $role", ['roles' => array_column($this->roles->all(), 'name')]);
        $user = $this->users->byId($userId) ?? throw new ApiError(404, 'not_found', 'User not found');
        $changed = $grant ? $this->roles->grant((string) $user['id'], $role, $grantedBy) : $this->roles->revoke((string) $user['id'], $role);
        return ['changed' => $changed, 'user' => $this->users->adminJson($this->users->byId((string) $user['id']) ?? $user)];
    }
}
