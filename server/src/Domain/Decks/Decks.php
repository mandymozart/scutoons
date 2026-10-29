<?php
declare(strict_types=1);

namespace Scutoons\Domain\Decks;

use ParagonIE_Sodium_Compat;
use Scutoons\Domain\Accounts\Users;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Mail\Mailer;
use Scutoons\Domain\Mail\MailTemplates;
use Scutoons\Domain\RateLimit\RateLimiter;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Ids;
use Throwable;

/**
 * Cyberdecks: identity and access (#180, agents/research/CYBERDECKS.md §9, owner 2026-10-29) – handshake, then
 * approval, then data. A deck has its own Ed25519 key pair (made at first boot; the private key never leaves it, no
 * shared secret in the image). It asks for access with its public key (`request`) and shows the short code on its
 * splash; a signed-in member – every member, readers included – approves the code on the account page (`approve`) and
 * the deck becomes theirs: exactly the member's rights, read-only (Http\DeckTokenScope). The approval's lifetime is set
 * there like the AI connections (Auth\ApiTokens): LIFETIME_DAYS by default, or shorter, optionally ending after
 * `idle_days` without the deck; renewable on the account page, one reminder mail in the last REMIND_DAYS (no cron:
 * sent when the deck renews its token). Within the approval the deck signs a server challenge (`challenge` → `token`)
 * and gets a short token (`scd_…`, TOKEN_SECONDS, only its SHA-256 stored), which it renews by itself. Revoked or
 * expired → the next renewal answers 401 and the deck asks again: the same row goes back to pending with a new code
 * (one row per public key), the build never becomes useless.
 *
 * The signed message: SIGNED_PREFIX . challenge (hex) . ':' . public key (hex) – domain-separated, bound to the deck.
 */
final class Decks
{
    public const TOKEN_PREFIX = 'scd_';
    public const SIGNED_PREFIX = 'scutoons-deck-token:v1:';
    /** a pending request ends after this (the code on the splash) */
    public const PENDING_MINUTES = 10;
    public const CHALLENGE_SECONDS = 120;
    public const TOKEN_SECONDS = 3600;
    public const LIFETIME_DAYS = 182;
    public const REMIND_DAYS = 14;
    /** approved decks per member (a forgotten one is revoked, not kept) */
    public const MAX = 50;
    /** wrong codes a member may try per hour (code guessing) */
    public const CODE_TRIES = 20;
    /** the code's alphabet: no 0 / O, 1 / I / L */
    private const ALPHABET = 'ABCDEFGHJKMNPQRSTUVWXYZ23456789';
    private const CODE_LENGTH = 6;

    /** @var array<string, array<string, mixed>|null> */
    private array $cache = [];

    public function __construct(
        private readonly Database $db,
        private readonly Clock $clock,
        private readonly Users $users,
        private readonly Mailer $mailer,
        private readonly Settings $settings,
        private readonly RateLimiter $limiter,
    ) {
    }

    public static function isDeckToken(?string $token): bool
    {
        return $token !== null && str_starts_with($token, self::TOKEN_PREFIX);
    }

    /** Before migration 2026-10-29-decks the endpoints answer 503 */
    private function requireAvailable(): void
    {
        try {
            $this->db->value('SELECT COUNT(*) FROM decks WHERE 1 = 0');
        } catch (Throwable) {
            throw new ApiError(503, 'not_available', 'Decks are not set up yet (migration 2026-10-29-decks).');
        }
    }

    // ── the deck's side (no session: its key is its identity) ────────────────────────────────────────────────

    /**
     * A deck asks for access → { status: pending, id, code, expires_at } (the same code again while its request is open),
     * or { status: approved, id } when its approval still holds (then it goes on with `challenge`)
     * @param array<mixed> $body { public_key, build?, class?, hostname? }
     * @return array<string, mixed>
     */
    public function request(array $body): array
    {
        $this->requireAvailable();
        $key = self::publicKey($body['public_key'] ?? null);
        $build = self::text($body['build'] ?? '', 64, 'build');
        $class = self::text($body['class'] ?? '', 16, 'class');
        $hostname = self::text($body['hostname'] ?? '', 64, 'hostname');
        $this->sweep();
        $row = $this->byKey($key);
        if ($row !== null && $this->valid($row)) return ['status' => 'approved', 'id' => (string) $row['id']];
        if ($row !== null && $row['status'] === 'pending' && $row['expires_at'] > $this->clock->utc()) {
            $this->db->run('UPDATE decks SET build = ?, class = ?, hostname = ? WHERE id = ?', [$build, $class, $hostname, $row['id']]);
            return self::pendingAnswer($row);
        }
        $code = $this->newCode();
        $expires = $this->clock->utc(self::PENDING_MINUTES * 60);
        if ($row === null) {
            $id = Ids::uuid7();
            $this->db->run('INSERT INTO decks (id, public_key, status, code, build, class, hostname, expires_at, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)',
                [$id, $key, 'pending', $code, $build, $class, $hostname, $expires, $this->clock->utc()]);
        } else {
            // revoked or expired: the same deck asks again – back to pending, nothing of the old approval stays
            $id = (string) $row['id'];
            $this->db->run('UPDATE decks SET status = ?, code = ?, build = ?, class = ?, hostname = ?, name = NULL, user_id = NULL, mapping = ?, mapping_ref = NULL,'
                . ' lifetime_days = NULL, idle_days = NULL, expires_at = ?, reminded_at = NULL, challenge = NULL, challenge_expires_at = NULL, token_hash = NULL,'
                . ' token_expires_at = NULL, created_at = ?, approved_at = NULL, revoked_at = NULL WHERE id = ?',
                ['pending', $code, $build, $class, $hostname, 'member', $expires, $this->clock->utc(), $id]);
        }
        $this->cache = [];
        return ['status' => 'pending', 'id' => $id, 'code' => $code, 'expires_at' => $expires];
    }

    /**
     * The deck polls: pending → { status: pending, code, expires_at }; approved → { status: approved, challenge,
     * expires_at } (a fresh single-use challenge to sign); anything else → 401 (unknown_deck, request_expired, revoked,
     * expired): the deck asks again with `request`
     * @return array<string, mixed>
     */
    public function challenge(mixed $publicKey): array
    {
        $this->requireAvailable();
        $row = $this->usable(self::publicKey($publicKey), true);
        if ($row['status'] === 'pending') return self::pendingAnswer($row);
        $challenge = bin2hex(random_bytes(32));
        $expires = $this->clock->utc(self::CHALLENGE_SECONDS);
        $this->db->run('UPDATE decks SET challenge = ?, challenge_expires_at = ? WHERE id = ?', [$challenge, $expires, $row['id']]);
        return ['status' => 'approved', 'challenge' => $challenge, 'expires_at' => $expires];
    }

    /**
     * The signed challenge → a short token: { token, expires_at, approval_expires_at, deck: { id, name } }. The challenge
     * is used up by any attempt; 401 bad_challenge / bad_signature, or the approval's 401s as in `challenge`
     * @param array<mixed> $body { public_key, challenge, signature } (signature: 64 bytes as hex)
     * @return array<string, mixed>
     */
    public function token(array $body): array
    {
        $this->requireAvailable();
        $key = self::publicKey($body['public_key'] ?? null);
        $row = $this->usable($key, false);
        $challenge = $body['challenge'] ?? null;
        $signature = $body['signature'] ?? null;
        $open = $row['challenge'];
        $this->db->run('UPDATE decks SET challenge = NULL, challenge_expires_at = NULL WHERE id = ?', [$row['id']]);
        if ($open === null || !is_string($challenge) || !hash_equals((string) $open, $challenge) || $row['challenge_expires_at'] < $this->clock->utc()) {
            throw new ApiError(401, 'bad_challenge', 'Unknown or expired challenge – ask for a new one.');
        }
        if (!is_string($signature) || !preg_match('/^[0-9a-fA-F]{128}$/', $signature)
            || !self::verify((string) hex2bin($signature), self::SIGNED_PREFIX . $challenge . ':' . $key, (string) hex2bin($key))) {
            throw new ApiError(401, 'bad_signature', 'The signature does not match this deck.');
        }
        $token = self::TOKEN_PREFIX . Ids::token();
        $expires = $this->clock->utc(self::TOKEN_SECONDS);
        $this->db->run('UPDATE decks SET token_hash = ?, token_expires_at = ?, last_seen_at = ? WHERE id = ?',
            [hash('sha256', $token), $expires, $this->clock->utc(), $row['id']]);
        $this->cache = [];
        $this->remind($row);
        return [
            'token' => $token,
            'expires_at' => $expires,
            'approval_expires_at' => (string) $row['expires_at'],
            'deck' => ['id' => (string) $row['id'], 'name' => self::displayName($row)],
        ];
    }

    /**
     * The member behind a deck token: null when unknown, its token ran out, or the approval was revoked / ended
     * @return array<string, mixed>|null
     */
    public function user(string $token): ?array
    {
        if (array_key_exists($token, $this->cache)) return $this->cache[$token];
        try {
            $row = $this->db->one('SELECT * FROM decks WHERE token_hash = ?', [hash('sha256', $token)]);
        } catch (Throwable) {
            return $this->cache[$token] = null; // before the migration: no deck token works
        }
        if ($row === null || !$this->valid($row) || $row['token_expires_at'] < $this->clock->utc()) return $this->cache[$token] = null;
        $user = $this->users->byId((string) $row['user_id']);
        if ($user === null) return $this->cache[$token] = null;
        if ($row['last_seen_at'] === null || $row['last_seen_at'] < $this->clock->utc(-60)) {
            $this->db->run('UPDATE decks SET last_seen_at = ? WHERE id = ?', [$this->clock->utc(), $row['id']]);
        }
        return $this->cache[$token] = $user;
    }

    // ── the member's side (a real sign-in; DeckTokenScope / ApiTokenScope refuse tokens) ───────────────────────

    /**
     * What waits behind a code (the pairing page shows it before the member approves): { code, hostname, build, class,
     * expires_at } · 404 unknown_code (counted against CODE_TRIES)
     * @param array<string, mixed> $user
     * @return array<string, mixed>
     */
    public function pending(array $user, mixed $code): array
    {
        $this->requireAvailable();
        $row = $this->pendingByCode($user, $code);
        return ['code' => (string) $row['code'], 'hostname' => (string) $row['hostname'], 'build' => (string) $row['build'],
            'class' => (string) $row['class'], 'expires_at' => (string) $row['expires_at']];
    }

    /**
     * The member approves the deck behind a code: it is theirs (mapping member) for `days` (1–182, default 182), optionally
     * ending after `idle_days` without it → the item
     * @param array<string, mixed> $user
     * @param array<mixed> $body { code, days?, idle_days?, name? }
     * @return array<string, mixed>
     */
    public function approve(array $user, array $body): array
    {
        $this->requireAvailable();
        $days = self::days($body['days'] ?? self::LIFETIME_DAYS, 'days');
        $idle = isset($body['idle_days']) ? self::days($body['idle_days'], 'idle_days') : null;
        $name = self::text($body['name'] ?? '', 100, 'name');
        $row = $this->pendingByCode($user, $body['code'] ?? null);
        $count = (int) $this->db->value('SELECT COUNT(*) FROM decks WHERE user_id = ? AND status = ?', [(string) $user['id'], 'approved']);
        if ($count >= self::MAX) throw new ApiError(409, 'too_many', 'At most ' . self::MAX . ' decks – revoke one you no longer use.');
        $now = $this->clock->utc();
        $this->db->run('UPDATE decks SET status = ?, code = NULL, user_id = ?, name = ?, mapping = ?, lifetime_days = ?, idle_days = ?, expires_at = ?,'
            . ' reminded_at = NULL, approved_at = ?, last_seen_at = ? WHERE id = ? AND status = ?',
            ['approved', (string) $user['id'], $name === '' ? null : $name, 'member', $days, $idle, $this->clock->utc($days * 86400), $now, $now, $row['id'], 'pending']);
        $this->cache = [];
        return $this->item($this->db->one('SELECT * FROM decks WHERE id = ?', [$row['id']]) ?? []);
    }

    /**
     * The member's decks (approved, also expired ones – renewable), newest first
     * @param array<string, mixed> $user
     * @return list<array<string, mixed>>
     */
    public function list(array $user): array
    {
        $this->requireAvailable();
        return array_map(fn(array $r) => $this->item($r),
            $this->db->all('SELECT * FROM decks WHERE user_id = ? AND status = ? ORDER BY approved_at DESC', [(string) $user['id'], 'approved']));
    }

    /**
     * Its lifetime once more from now (also an expired one; an idle window starts again); the reminder may come again
     * @param array<string, mixed> $user
     * @return array<string, mixed>
     */
    public function renew(array $user, string $id): array
    {
        $this->requireAvailable();
        $row = $this->own($user, $id);
        $days = (int) ($row['lifetime_days'] ?? self::LIFETIME_DAYS);
        $this->db->run('UPDATE decks SET expires_at = ?, approved_at = ?, reminded_at = NULL WHERE id = ?', [$this->clock->utc($days * 86400), $this->clock->utc(), $row['id']]);
        $this->cache = [];
        return $this->item($this->db->one('SELECT * FROM decks WHERE id = ?', [$row['id']]) ?? []);
    }

    /**
     * Revoked: its token stops at once, its next renewal answers 401 (it asks again from the splash)
     * @param array<string, mixed> $user
     */
    public function revoke(array $user, string $id): void
    {
        $this->requireAvailable();
        $row = $this->own($user, $id);
        $this->db->run('UPDATE decks SET status = ?, revoked_at = ?, token_hash = NULL, token_expires_at = NULL, challenge = NULL, challenge_expires_at = NULL WHERE id = ?',
            ['revoked', $this->clock->utc(), $row['id']]);
        $this->cache = [];
    }

    // ── helpers ───────────────────────────────────────────────────────────────────────────────────────────

    /** An Ed25519 detached signature (ext-sodium when there, else sodium_compat's PHP) */
    public static function verify(string $signature, string $message, string $publicKey): bool
    {
        if (strlen($signature) !== 64 || strlen($publicKey) !== 32) return false;
        try {
            return ParagonIE_Sodium_Compat::crypto_sign_verify_detached($signature, $message, $publicKey);
        } catch (Throwable) {
            return false;
        }
    }

    /**
     * Approved, not past its end, and seen within its idle window
     * @param array<string, mixed> $row
     */
    private function valid(array $row): bool
    {
        if ($row['status'] !== 'approved' || $row['expires_at'] <= $this->clock->utc()) return false;
        if ($row['idle_days'] === null) return true;
        $seen = max((string) ($row['last_seen_at'] ?? ''), (string) ($row['approved_at'] ?? ''));
        return $seen > $this->clock->utc(-(int) $row['idle_days'] * 86400);
    }

    /**
     * The deck's row while it may go on (pending with an open request when $pendingOk, else approved and valid) – else 401
     * @return array<string, mixed>
     */
    private function usable(string $key, bool $pendingOk): array
    {
        $row = $this->byKey($key) ?? throw new ApiError(401, 'unknown_deck', 'Unknown deck – ask for access first.');
        if ($row['status'] === 'pending') {
            if ($row['expires_at'] <= $this->clock->utc()) throw new ApiError(401, 'request_expired', 'The request ran out – ask again.');
            if ($pendingOk) return $row;
            throw new ApiError(401, 'not_approved', 'Not approved yet.');
        }
        if ($row['status'] === 'revoked') throw new ApiError(401, 'revoked', 'This deck was revoked – ask again.');
        if (!$this->valid($row)) throw new ApiError(401, 'expired', 'This deck\'s approval ended – ask again.');
        return $row;
    }

    /**
     * @param array<string, mixed> $user
     * @return array<string, mixed>
     */
    private function pendingByCode(array $user, mixed $code): array
    {
        $bucket = 'deckcode:' . (string) $user['id'];
        $this->limiter->guard($bucket, self::CODE_TRIES, 3600, 'Too many wrong codes – try again later.');
        $code = strtoupper((string) preg_replace('/[^A-Za-z0-9]/', '', is_string($code) ? $code : ''));
        $row = strlen($code) === self::CODE_LENGTH
            ? $this->db->one('SELECT * FROM decks WHERE code = ? AND status = ? AND expires_at > ?', [$code, 'pending', $this->clock->utc()])
            : null;
        if ($row === null) {
            $this->limiter->hit($bucket);
            throw new ApiError(404, 'unknown_code', 'No deck waits with this code – check the code on its screen (it changes after ' . self::PENDING_MINUTES . ' minutes).', ['field' => 'code']);
        }
        return $row;
    }

    /**
     * @param array<string, mixed> $user
     * @return array<string, mixed>
     */
    private function own(array $user, string $id): array
    {
        return $this->db->one('SELECT * FROM decks WHERE id = ? AND user_id = ? AND status = ?', [$id, (string) $user['id'], 'approved'])
            ?? throw new ApiError(404, 'not_found', 'No such deck (revoked already?).');
    }

    /** @return array<string, mixed>|null */
    private function byKey(string $key): ?array
    {
        return $this->db->one('SELECT * FROM decks WHERE public_key = ?', [$key]);
    }

    /** Pending requests that ran out are gone (their codes free again) */
    private function sweep(): void
    {
        $this->db->run('DELETE FROM decks WHERE status = ? AND expires_at < ?', ['pending', $this->clock->utc()]);
    }

    private function newCode(): string
    {
        for ($try = 0; $try < 8; $try++) {
            $code = '';
            for ($i = 0; $i < self::CODE_LENGTH; $i++) $code .= self::ALPHABET[random_int(0, strlen(self::ALPHABET) - 1)];
            if ($this->db->value('SELECT 1 FROM decks WHERE code = ?', [$code]) === null) return $code;
        }
        throw new ApiError(503, 'busy', 'Too many decks waiting – try again in a minute.');
    }

    /**
     * Its last REMIND_DAYS and no reminder yet this period: one mail to the member (marked first; a failed mail is logged)
     * @param array<string, mixed> $row
     */
    private function remind(array $row): void
    {
        if (($row['reminded_at'] ?? null) !== null || $row['expires_at'] > $this->clock->utc(self::REMIND_DAYS * 86400)) return;
        $user = $this->users->byId((string) $row['user_id']);
        if ($user === null) return;
        try {
            $this->db->run('UPDATE decks SET reminded_at = ? WHERE id = ? AND reminded_at IS NULL', [$this->clock->utc(), $row['id']]);
            $this->mailer->send(MailTemplates::DECK_EXPIRING, (string) $user['email'], (string) ($user['locale'] ?? 'en'), [
                'name' => self::displayName($row),
                'date' => substr((string) $row['expires_at'], 0, 10),
                'url' => rtrim($this->settings->get('APP_URL'), '/') . '/account',
            ]);
        } catch (Throwable $error) {
            error_log('[qr-api] deck_expiring: ' . $error->getMessage());
        }
    }

    /**
     * @param array<string, mixed> $row
     * @return array<string, mixed>
     */
    private static function pendingAnswer(array $row): array
    {
        return ['status' => 'pending', 'id' => (string) $row['id'], 'code' => (string) $row['code'], 'expires_at' => (string) $row['expires_at']];
    }

    /** @param array<string, mixed> $row */
    private static function displayName(array $row): string
    {
        $name = (string) ($row['name'] ?? '');
        return $name !== '' ? $name : ((string) $row['hostname'] !== '' ? (string) $row['hostname'] : 'Deck');
    }

    /**
     * @param array<string, mixed> $r
     * @return array<string, mixed>
     */
    private function item(array $r): array
    {
        return [
            'id' => (string) $r['id'],
            'name' => self::displayName($r),
            'hostname' => (string) $r['hostname'],
            'build' => (string) $r['build'],
            'class' => (string) $r['class'],
            'mapping' => (string) $r['mapping'],
            'approved_at' => $r['approved_at'] !== null ? (string) $r['approved_at'] : null,
            'last_seen_at' => $r['last_seen_at'] !== null ? (string) $r['last_seen_at'] : null,
            'expires_at' => (string) $r['expires_at'],
            'lifetime_days' => $r['lifetime_days'] !== null ? (int) $r['lifetime_days'] : null,
            'idle_days' => $r['idle_days'] !== null ? (int) $r['idle_days'] : null,
            'expired' => !$this->valid($r),
        ];
    }

    private static function publicKey(mixed $value): string
    {
        if (!is_string($value) || !preg_match('/^[0-9a-fA-F]{64}$/', $value)) {
            throw new ApiError(400, 'invalid_public_key', 'public_key is the deck\'s Ed25519 public key: 32 bytes as hex.', ['field' => 'public_key']);
        }
        return strtolower($value);
    }

    private static function text(mixed $value, int $max, string $field): string
    {
        if (!is_string($value)) throw new ApiError(400, 'invalid_' . $field, $field . ' is text.', ['field' => $field]);
        $value = trim((string) preg_replace('/[\x00-\x1F\x7F\s]+/u', ' ', $value));
        if (mb_strlen($value) > $max) throw new ApiError(400, 'invalid_' . $field, 'At most ' . $max . ' characters.', ['field' => $field]);
        return $value;
    }

    private static function days(mixed $value, string $field): int
    {
        if (!is_int($value) || $value < 1 || $value > self::LIFETIME_DAYS) {
            throw new ApiError(400, 'invalid_' . $field, $field . ' is a whole number of days, 1 to ' . self::LIFETIME_DAYS . '.', ['field' => $field]);
        }
        return $value;
    }
}
