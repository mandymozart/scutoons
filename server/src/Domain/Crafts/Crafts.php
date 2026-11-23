<?php
declare(strict_types=1);

namespace Scutoons\Domain\Crafts;

use PDOException;
use Scutoons\Domain\Accounts\Users;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Ids;
use Scutoons\Support\Time;

/**
 * Crafts (#826, migration 2026-11-24-crafts): what a member does – illustrator, writer, translator … – and so which creator
 * workspaces they start with (`crafts.workspaces`, the creator's workspace ids; the creator reads them, it keeps no copy).
 * Not the catalog recipes' "crafting" (items made from items, Domain\Catalog): a different word on purpose.
 *
 * Rights (the API decides): a member picks their own crafts once, at the first login (`crafts_asked_at` still empty);
 * afterwards only an admin changes them – on the user page or in an invite. An admin's invite by address carries crafts,
 * signs that address up without a code (LoginService) and applies its crafts when the account comes.
 */
final class Crafts
{
    public const SOURCES = ['invite', 'admin', 'member'];

    public function __construct(private readonly Database $db, private readonly Clock $clock, private readonly Roles $roles, private readonly Users $users)
    {
    }

    /**
     * Every craft in its order: [{ id, name: locale map, icon, workspaces }]; [] before the migration ran
     * @return list<array{id: string, name: array<string, string>, icon: string, workspaces: list<string>}>
     */
    public function all(): array
    {
        try {
            $rows = $this->db->all('SELECT id, name, icon, workspaces FROM crafts ORDER BY position, id');
        } catch (PDOException $error) {
            if (self::missing($error)) return [];
            throw $error;
        }
        return array_map(static fn(array $r) => [
            'id' => (string) $r['id'],
            'name' => self::stringMap(json_decode((string) $r['name'], true)),
            'icon' => (string) $r['icon'],
            'workspaces' => self::stringList(json_decode((string) $r['workspaces'], true)),
        ], $rows);
    }

    /**
     * The member's crafts with where each came from, in the crafts' order
     * @return list<array{id: string, source: string}>
     */
    public function of(string $userId): array
    {
        try {
            $rows = $this->db->all('SELECT m.craft_id, m.source FROM member_crafts m JOIN crafts c ON c.id = m.craft_id WHERE m.user_id = ? ORDER BY c.position, c.id', [$userId]);
        } catch (PDOException $error) {
            if (self::missing($error)) return [];
            throw $error;
        }
        return array_map(static fn(array $r) => ['id' => (string) $r['craft_id'], 'source' => (string) $r['source']], $rows);
    }

    /**
     * GET /api/me/crafts: the member's crafts, the workspaces they start with (the server resolves the map, in the crafts'
     * order, each once), whether the first-login question is still open, whether the member may change them now.
     * @param array<string, mixed> $user
     * @return array{crafts: list<string>, invited: list<string>, workspaces: list<string>, ask: bool, can_edit: bool}
     */
    public function state(array $user): array
    {
        $userId = (string) $user['id'];
        $mine = $this->of($userId);
        $ids = array_column($mine, 'id');
        $asked = $this->askedAt($userId);
        $available = $asked !== false;
        $ask = $available && $asked === null;
        return [
            'crafts' => $ids,
            'invited' => array_values(array_column(array_filter($mine, static fn(array $c) => $c['source'] === 'invite'), 'id')),
            'workspaces' => $this->workspacesOf($ids),
            'ask' => $ask,
            'can_edit' => $available && ($ask || $this->roles->has($user, Roles::ADMIN)),
        ];
    }

    /**
     * PUT /api/me/crafts { crafts }: the member's own answer – only while the first-login question is open, else only an
     * admin (403 crafts_locked). Marks the question answered (also with no craft picked).
     * @param array<string, mixed> $user
     * @param array<string, mixed> $body
     * @return array{crafts: list<string>, invited: list<string>, workspaces: list<string>, ask: bool, can_edit: bool}
     */
    public function setOwn(array $user, array $body): array
    {
        $ids = $this->validIds($body['crafts'] ?? null);
        $userId = (string) $user['id'];
        $asked = $this->askedAt($userId);
        if ($asked === false) throw new ApiError(503, 'not_available', 'Crafts are not set up yet.');
        if ($asked !== null && !$this->roles->has($user, Roles::ADMIN)) {
            throw new ApiError(403, 'crafts_locked', 'An admin changes your crafts.');
        }
        $this->db->transaction(function () use ($userId, $ids) {
            $this->replace($userId, $ids, 'member', null);
            $this->db->run('UPDATE users SET crafts_asked_at = COALESCE(crafts_asked_at, ?) WHERE id = ?', [$this->clock->utc(), $userId]);
        });
        return $this->state($user);
    }

    /**
     * An admin sets an account's crafts (the user page). The first-login question stays as it is: a member not asked yet
     * still is, with these ticked.
     * @return list<array{id: string, source: string}>
     */
    public function setFor(string $userId, mixed $crafts, string $by): array
    {
        $ids = $this->validIds($crafts);
        $this->db->transaction(fn() => $this->replace($userId, $ids, 'admin', $by));
        return $this->of($userId);
    }

    // ── invites ─────────────────────────────────────────────────────────────────────────────────────────────────────

    /**
     * The admin's invites, open first, then accepted (newest first)
     * @return list<array<string, mixed>>
     */
    public function invites(): array
    {
        try {
            $rows = $this->db->all('SELECT * FROM member_invites ORDER BY accepted_at IS NOT NULL, created_at DESC, id LIMIT 500');
        } catch (PDOException $error) {
            if (self::missing($error)) return [];
            throw $error;
        }
        return array_map(self::inviteJson(...), $rows);
    }

    /**
     * POST /api/admin/member-invites { email, crafts }: invite an address (one open invite per address – a second one
     * replaces its crafts). 409 has_account when the address has an account (its crafts are set on the user page).
     * @param array<string, mixed> $body
     * @return array<string, mixed>
     */
    public function invite(array $body, string $by): array
    {
        $email = Users::normalizeEmail($body['email'] ?? null) ?? throw new ApiError(400, 'invalid_email', 'This email address is not valid.');
        $ids = $this->validIds($body['crafts'] ?? null);
        if ($this->users->byEmail($email) !== null) throw new ApiError(409, 'has_account', 'This address has an account – set its crafts on the user page.');
        $open = $this->db->one('SELECT id, accepted_at FROM member_invites WHERE email = ?', [$email]);
        $json = (string) json_encode($ids);
        if ($open) {
            $this->db->run('UPDATE member_invites SET crafts = ?, invited_by = ?, created_at = ?, accepted_at = NULL, user_id = NULL WHERE id = ?', [$json, $by, $this->clock->utc(), $open['id']]);
            $id = (string) $open['id'];
        } else {
            $id = Ids::uuid();
            $this->db->run('INSERT INTO member_invites (id, email, crafts, invited_by, created_at) VALUES (?, ?, ?, ?, ?)', [$id, $email, $json, $by, $this->clock->utc()]);
        }
        return self::inviteJson($this->db->one('SELECT * FROM member_invites WHERE id = ?', [$id]) ?? []);
    }

    /** DELETE /api/admin/member-invites/{id}: only while open (404 / 409 accepted) */
    public function withdraw(string $id): void
    {
        $row = $this->db->one('SELECT accepted_at FROM member_invites WHERE id = ?', [$id]) ?? throw new ApiError(404, 'not_found', 'Invite not found.');
        if ($row['accepted_at'] !== null) throw new ApiError(409, 'accepted', 'This invite was accepted – change the crafts on the user page.');
        $this->db->run('DELETE FROM member_invites WHERE id = ? AND accepted_at IS NULL', [$id]);
    }

    /** An open admin invite waits for this address: it signs the address up (LoginService) */
    public function invited(string $email): bool
    {
        try {
            return $this->db->value('SELECT 1 FROM member_invites WHERE email = ? AND accepted_at IS NULL', [strtolower(trim($email))]) !== null;
        } catch (PDOException $error) {
            if (self::missing($error)) return false;
            throw $error;
        }
    }

    /**
     * The account for an invited address came (or signed in): the invite is accepted and its crafts are the member's,
     * unless the member already answered the question. Runs inside the sign-in's transaction.
     * @param array<string, mixed> $user
     */
    public function accept(array $user): void
    {
        try {
            $row = $this->db->one('SELECT * FROM member_invites WHERE email = ? AND accepted_at IS NULL', [strtolower(trim((string) $user['email']))]);
        } catch (PDOException $error) {
            if (self::missing($error)) return;
            throw $error;
        }
        if (!$row) return;
        $userId = (string) $user['id'];
        $this->db->run('UPDATE member_invites SET accepted_at = ?, user_id = ? WHERE id = ?', [$this->clock->utc(), $userId, $row['id']]);
        if ($this->askedAt($userId) !== null) return;
        $known = array_column($this->all(), 'id');
        $ids = array_values(array_intersect($known, self::stringList(json_decode((string) $row['crafts'], true))));
        $this->replace($userId, $ids, 'invite', $row['invited_by'] !== null ? (string) $row['invited_by'] : null);
    }

    // ── helpers ─────────────────────────────────────────────────────────────────────────────────────────────────────

    /**
     * The workspaces the crafts start with: in the crafts' order, each once
     * @param list<string> $ids
     * @return list<string>
     */
    private function workspacesOf(array $ids): array
    {
        $out = [];
        foreach ($this->all() as $craft) {
            if (!in_array($craft['id'], $ids, true)) continue;
            foreach ($craft['workspaces'] as $w) if (!in_array($w, $out, true)) $out[] = $w;
        }
        return $out;
    }

    /** @param list<string> $ids */
    private function replace(string $userId, array $ids, string $source, ?string $by): void
    {
        $this->db->run('DELETE FROM member_crafts WHERE user_id = ?', [$userId]);
        foreach ($ids as $id) {
            $this->db->run('INSERT INTO member_crafts (user_id, craft_id, source, assigned_by, created_at) VALUES (?, ?, ?, ?, ?)', [$userId, $id, $source, $by, $this->clock->utc()]);
        }
    }

    /**
     * A list of known craft ids, each once, in the crafts' order (400 invalid_crafts / unknown_craft)
     * @return list<string>
     */
    private function validIds(mixed $value): array
    {
        if (!is_array($value) || !array_is_list($value)) throw new ApiError(400, 'invalid_crafts', 'crafts must be a list of craft ids.');
        $known = array_column($this->all(), 'id');
        foreach ($value as $id) {
            if (!is_string($id) || !in_array($id, $known, true)) throw new ApiError(400, 'unknown_craft', 'Unknown craft.', ['craft' => is_scalar($id) ? (string) $id : null]);
        }
        return array_values(array_intersect($known, $value));
    }

    /** null: not asked yet · string: when · false: the migration has not run */
    private function askedAt(string $userId): string|false|null
    {
        try {
            $value = $this->db->value('SELECT crafts_asked_at FROM users WHERE id = ?', [$userId]);
        } catch (PDOException $error) {
            if (self::missing($error)) return false;
            throw $error;
        }
        return $value === null ? null : (string) $value;
    }

    /**
     * @param array<string, mixed> $r
     * @return array<string, mixed>
     */
    private static function inviteJson(array $r): array
    {
        return [
            'id' => (string) ($r['id'] ?? ''),
            'email' => (string) ($r['email'] ?? ''),
            'crafts' => self::stringList(json_decode((string) ($r['crafts'] ?? '[]'), true)),
            'invited_by' => $r['invited_by'] ?? null,
            'created_at' => Time::iso($r['created_at'] ?? null),
            'accepted_at' => Time::iso($r['accepted_at'] ?? null),
            'user_id' => $r['user_id'] ?? null,
        ];
    }

    /** @return list<string> */
    private static function stringList(mixed $v): array
    {
        return is_array($v) ? array_values(array_filter($v, 'is_string')) : [];
    }

    /** @return array<string, string> */
    private static function stringMap(mixed $v): array
    {
        if (!is_array($v)) return [];
        $out = [];
        foreach ($v as $k => $s) if (is_string($k) && is_string($s)) $out[$k] = $s;
        return $out;
    }

    /** MariaDB 42S02 (table) / 42S22 (column): the migration has not run yet */
    private static function missing(PDOException $error): bool
    {
        return in_array($error->getCode(), ['42S02', '42S22'], true) || preg_match('/Base table or view not found|Unknown column/i', $error->getMessage()) === 1;
    }
}
