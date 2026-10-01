<?php
declare(strict_types=1);

namespace Scutoons\Domain\Auth;

use Scutoons\Domain\Accounts\Users;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Ids;

/**
 * Session tokens (Bearer header or the single sign-on cookie, Http\SessionCookie). Only the SHA-256 of a token is stored. A session ends after SESSION_TTL_DAYS without use;
 * last_used_at is refreshed at most hourly.
 */
final class Sessions
{
    /** @var array<string, array<string, mixed>|null> token → user (per request) */
    private array $cache = [];

    public function __construct(
        private readonly Database $db,
        private readonly Clock $clock,
        private readonly Settings $settings,
        private readonly Users $users,
    ) {
    }

    /** A new session; returns the token */
    public function create(string $userId): string
    {
        $token = Ids::token();
        $this->db->run('INSERT INTO sessions (id, user_id, token_hash, created_at, last_used_at) VALUES (?, ?, ?, ?, ?)',
            [Ids::uuid(), $userId, hash('sha256', $token), $this->clock->utc(), $this->clock->utc()]);
        return $token;
    }

    /** @return array<string, mixed>|null the user of a valid session token */
    public function user(string $token): ?array
    {
        if (array_key_exists($token, $this->cache)) return $this->cache[$token];
        $session = $this->db->one('SELECT * FROM sessions WHERE token_hash = ?', [hash('sha256', $token)]);
        if (!$session) return $this->cache[$token] = null;
        if ($session['last_used_at'] < $this->clock->utc(-86400 * $this->settings->int('SESSION_TTL_DAYS'))) {
            $this->db->run('DELETE FROM sessions WHERE id = ?', [$session['id']]);
            return $this->cache[$token] = null;
        }
        if ($session['last_used_at'] < $this->clock->utc(-3600)) {
            $this->db->run('UPDATE sessions SET last_used_at = ? WHERE id = ?', [$this->clock->utc(), $session['id']]);
        }
        return $this->cache[$token] = $this->users->byId((string) $session['user_id']);
    }

    /** Ends one session (logout on this device) */
    public function end(string $token): void
    {
        $this->db->run('DELETE FROM sessions WHERE token_hash = ?', [hash('sha256', $token)]);
        unset($this->cache[$token]);
    }

    /** Ends every session of an account ("sign out on all devices") */
    public function endAll(string $userId): void
    {
        $this->db->run('DELETE FROM sessions WHERE user_id = ?', [$userId]);
        $this->cache = [];
    }

    /** `Authorization: Bearer <token>` header value → the token, or null when missing/malformed */
    public static function tokenFromHeader(string $header): ?string
    {
        return preg_match('/^Bearer\s+([A-Za-z0-9_-]{20,128})$/', trim($header), $m) ? $m[1] : null;
    }
}
