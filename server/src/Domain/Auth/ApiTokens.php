<?php
declare(strict_types=1);

namespace Scutoons\Domain\Auth;

use Scutoons\Domain\Accounts\Users;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Mail\Mailer;
use Scutoons\Domain\Mail\MailTemplates;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Ids;
use Throwable;

/**
 * Personal access tokens – "AI connections" (agents/MCP.md §3, owner 2026-10-04 / 2026-10-09): a member creates one on
 * their account page and gives it to their own AI (Claude Desktop / Claude Code start the SCUTOONS MCP server with it).
 * The API takes it as `Authorization: Bearer sct_…` like a session: the member's rights, their CURRENT roles. Only the
 * SHA-256 is stored; the token is shown once. Readers can't have one (owner 2026-10-09): creating needs a member role
 * (creator, liaison, ambassador, admin), and a token stops working once its account has none of them left. What a token
 * may NOT do (publish, send, apply, manage tokens …) the Session middleware refuses (Http\ApiTokenScope).
 *
 * Six months (owner 2026-10-09): a token expires LIFETIME_DAYS after it was made or last renewed; renewing (the account
 * page, a real sign-in) moves it six months on – the same token, nothing to set up again in the AI app. Used within its
 * last REMIND_DAYS, its member gets ONE reminder mail (ai_token_expiring) per period – no cron: a token nobody uses
 * needs no reminder.
 */
final class ApiTokens
{
    public const PREFIX = 'sct_';
    public const MEMBER_ROLES = ['creator', 'liaison', 'ambassador', Roles::ADMIN];
    public const NAME_MAX = 100;
    /** tokens per account (a forgotten one is revoked, not kept) */
    public const MAX = 20;
    public const LIFETIME_DAYS = 182;
    public const REMIND_DAYS = 14;

    /** @var array<string, array{user: array<string, mixed>|null, token: array{id: string, name: string}|null}> */
    private array $cache = [];

    public function __construct(
        private readonly Database $db,
        private readonly Clock $clock,
        private readonly Users $users,
        private readonly Roles $roles,
        private readonly Mailer $mailer,
        private readonly Settings $settings,
    ) {
    }

    public static function isApiToken(?string $token): bool
    {
        return $token !== null && str_starts_with($token, self::PREFIX);
    }

    /** Before migration 2026-10-09-api-tokens the endpoints answer 503 */
    private function requireAvailable(): void
    {
        try {
            $this->db->value('SELECT COUNT(*) FROM api_tokens WHERE 1 = 0');
        } catch (Throwable) {
            throw new ApiError(503, 'not_available', 'AI connections are not set up yet (migration 2026-10-09-api-tokens).');
        }
    }

    /** @param array<string, mixed> $user */
    public function isMember(array $user): bool
    {
        foreach (self::MEMBER_ROLES as $role) {
            if ($this->roles->has($user, $role)) return true;
        }
        return false;
    }

    /**
     * A new token for the signed-in member → `{ token, item }` (the token only here, never again)
     * @param array<string, mixed> $user
     * @return array{token: string, item: array<string, mixed>}
     */
    public function create(array $user, string $name): array
    {
        $this->requireAvailable();
        if (!$this->isMember($user)) throw new ApiError(403, 'no_member_role', 'AI connections are for members (creator, liaison, ambassador, admin).');
        $name = trim((string) preg_replace('/\s+/', ' ', $name));
        if ($name === '') throw new ApiError(400, 'name_required', 'Give the connection a name (e.g. "Claude on my laptop").', ['field' => 'name']);
        if (mb_strlen($name) > self::NAME_MAX) throw new ApiError(400, 'name_too_long', 'At most ' . self::NAME_MAX . ' characters.', ['field' => 'name']);
        $open = (int) $this->db->value('SELECT COUNT(*) FROM api_tokens WHERE user_id = ? AND revoked_at IS NULL', [(string) $user['id']]);
        if ($open >= self::MAX) throw new ApiError(409, 'too_many', 'At most ' . self::MAX . ' AI connections – revoke one you no longer use.');
        $token = self::PREFIX . Ids::token();
        $id = Ids::uuid7();
        $this->db->run('INSERT INTO api_tokens (id, user_id, name, token_hash, created_at, expires_at) VALUES (?, ?, ?, ?, ?, ?)',
            [$id, (string) $user['id'], $name, hash('sha256', $token), $this->clock->utc(), $this->clock->utc(self::LIFETIME_DAYS * 86400)]);
        return ['token' => $token, 'item' => $this->item($this->db->one('SELECT * FROM api_tokens WHERE id = ?', [$id]) ?? [])];
    }

    /**
     * The member's tokens that still work, newest first (no hashes)
     * @param array<string, mixed> $user
     * @return list<array<string, mixed>>
     */
    public function list(array $user): array
    {
        $this->requireAvailable();
        return array_map(fn(array $r) => $this->item($r),
            $this->db->all('SELECT * FROM api_tokens WHERE user_id = ? AND revoked_at IS NULL ORDER BY created_at DESC', [(string) $user['id']]));
    }

    /**
     * Six more months from now (also an expired one – it is the member's own, signed in); the reminder may come again
     * @param array<string, mixed> $user
     * @return array<string, mixed>
     */
    public function renew(array $user, string $id): array
    {
        $this->requireAvailable();
        $row = $this->db->one('SELECT * FROM api_tokens WHERE id = ? AND user_id = ? AND revoked_at IS NULL', [$id, (string) $user['id']])
            ?? throw new ApiError(404, 'not_found', 'No such AI connection (revoked already?).');
        $this->db->run('UPDATE api_tokens SET expires_at = ?, reminded_at = NULL WHERE id = ?', [$this->clock->utc(self::LIFETIME_DAYS * 86400), $row['id']]);
        $this->cache = [];
        return $this->item($this->db->one('SELECT * FROM api_tokens WHERE id = ?', [$row['id']]) ?? []);
    }

    /** @param array<string, mixed> $user */
    public function revoke(array $user, string $id): void
    {
        $this->requireAvailable();
        $row = $this->db->one('SELECT * FROM api_tokens WHERE id = ? AND user_id = ? AND revoked_at IS NULL', [$id, (string) $user['id']])
            ?? throw new ApiError(404, 'not_found', 'No such AI connection (revoked already?).');
        $this->db->run('UPDATE api_tokens SET revoked_at = ? WHERE id = ?', [$this->clock->utc(), $row['id']]);
        $this->cache = [];
    }

    /**
     * The account behind a token: null when unknown, revoked, expired or its account has no member role any more
     * @return array<string, mixed>|null
     */
    public function user(string $token): ?array
    {
        return $this->resolve($token)['user'];
    }

    /** @return array{id: string, name: string}|null the token's id and name (for the save message "(AI: <name>)") */
    public function token(string $token): ?array
    {
        return $this->resolve($token)['token'];
    }

    /** @return array{user: array<string, mixed>|null, token: array{id: string, name: string}|null} */
    private function resolve(string $token): array
    {
        if (array_key_exists($token, $this->cache)) return $this->cache[$token];
        $none = ['user' => null, 'token' => null];
        try {
            $row = $this->db->one('SELECT * FROM api_tokens WHERE token_hash = ?', [hash('sha256', $token)]);
        } catch (Throwable) {
            return $this->cache[$token] = $none; // before the migration: no token works
        }
        if (!$row || $row['revoked_at'] !== null) return $this->cache[$token] = $none;
        if ($row['expires_at'] !== null && $row['expires_at'] < $this->clock->utc()) return $this->cache[$token] = $none;
        $user = $this->users->byId((string) $row['user_id']);
        if ($user === null || !$this->isMember($user)) return $this->cache[$token] = $none;
        if ($row['last_used_at'] === null || $row['last_used_at'] < $this->clock->utc(-60)) {
            $this->db->run('UPDATE api_tokens SET last_used_at = ? WHERE id = ?', [$this->clock->utc(), $row['id']]);
        }
        $this->remind($row, $user);
        return $this->cache[$token] = ['user' => $user, 'token' => ['id' => (string) $row['id'], 'name' => (string) $row['name']]];
    }

    /**
     * Its last REMIND_DAYS and no reminder yet this period: one mail to the member (marked first, so parallel calls send
     * one; a failed mail is logged, the request goes on)
     * @param array<string, mixed> $row
     * @param array<string, mixed> $user
     */
    private function remind(array $row, array $user): void
    {
        if ($row['expires_at'] === null || ($row['reminded_at'] ?? null) !== null) return;
        if ($row['expires_at'] > $this->clock->utc(self::REMIND_DAYS * 86400)) return;
        try {
            $this->db->run('UPDATE api_tokens SET reminded_at = ? WHERE id = ? AND reminded_at IS NULL', [$this->clock->utc(), $row['id']]);
            $this->mailer->send(MailTemplates::AI_TOKEN_EXPIRING, (string) $user['email'], (string) ($user['locale'] ?? 'en'), [
                'name' => (string) $row['name'],
                'date' => substr((string) $row['expires_at'], 0, 10),
                'url' => rtrim($this->settings->get('APP_URL'), '/') . '/account',
            ]);
        } catch (Throwable $error) {
            error_log('[qr-api] ai_token_expiring: ' . $error->getMessage()); // before the expiry migration, or the mail failed
        }
    }

    /**
     * @param array<string, mixed> $r
     * @return array<string, mixed>
     */
    private function item(array $r): array
    {
        return [
            'id' => (string) $r['id'],
            'name' => (string) $r['name'],
            'created_at' => (string) $r['created_at'],
            'last_used_at' => $r['last_used_at'] !== null ? (string) $r['last_used_at'] : null,
            'expires_at' => $r['expires_at'] !== null ? (string) $r['expires_at'] : null,
            'expired' => $r['expires_at'] !== null && $r['expires_at'] < $this->clock->utc(),
        ];
    }
}
