<?php
declare(strict_types=1);

namespace Scutoons\Domain\Events;

use InvalidArgumentException;
use Scutoons\Domain\RequestContext;
use Scutoons\Domain\Secrets;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Database\Database;
use Throwable;

/**
 * Activity statistics per code (table code_events, append-only; agents/ITEMS.md "Revoke + activity statistics").
 * Events: scan (landing page /?c=), access (/access/, at most once per code + device per UTC day), claim,
 * claim_refused, login (one row per owned code at sign-in), signup (account created with this code), revoke, unrevoke.
 *
 * Privacy: no raw IP, email or user agent. device_hash = HMAC of the browser's random `X-Device-Id`; ip_hash = HMAC of
 * the IP with a daily salt derived from AUTH_SECRET + the UTC date (hashes of different days can't be linked).
 * Logging never breaks a request: failures only go to the error log.
 */
final class EventLog
{
    public const TYPES = ['scan', 'access', 'claim', 'claim_refused', 'login', 'signup', 'revoke', 'unrevoke'];

    public function __construct(
        private readonly Database $db,
        private readonly Clock $clock,
        private readonly Secrets $secrets,
        private readonly RequestContext $context,
    ) {
    }

    public function log(string $event, string $code, ?string $userId = null, ?string $toon = null): void
    {
        try {
            if (!in_array($event, self::TYPES, true)) throw new InvalidArgumentException("unknown event $event");
            $this->db->run('INSERT INTO code_events (code, event, device_hash, user_id, ip_hash, toon, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)',
                [substr($code, 0, 8), $event, $this->deviceHash(), $userId, $this->ipHash(), self::toon($toon), $this->clock->utc()]);
        } catch (Throwable $error) {
            error_log('[qr-api] event log failed: ' . $error->getMessage());
        }
    }

    /** `access` once per code + device (or, without a device id, per daily IP hash) per UTC day */
    public function logAccess(string $code, ?string $userId = null, ?string $toon = null): void
    {
        try {
            $device = $this->deviceHash();
            $dayStart = gmdate('Y-m-d', $this->clock->time()) . ' 00:00:00';
            $seen = $device !== null
                ? $this->db->value("SELECT 1 FROM code_events WHERE code = ? AND event = 'access' AND device_hash = ? AND created_at >= ? LIMIT 1", [$code, $device, $dayStart])
                : $this->db->value("SELECT 1 FROM code_events WHERE code = ? AND event = 'access' AND device_hash IS NULL AND ip_hash = ? AND created_at >= ? LIMIT 1", [$code, $this->ipHash(), $dayStart]);
            if ($seen === null) $this->log('access', $code, $userId, $toon);
        } catch (Throwable $error) {
            error_log('[qr-api] access log failed: ' . $error->getMessage());
        }
    }

    /** HMAC of the X-Device-Id header (random per browser, localStorage `scutoons_device`), or null */
    public function deviceHash(): ?string
    {
        $id = $this->context->deviceId();
        if ($id === null) return null;
        return hash_hmac('sha256', $id, $this->secrets->derivedKey('scutoons device id v1'));
    }

    /** HMAC of the client IP with today's salt (AUTH_SECRET + UTC date) */
    public function ipHash(): string
    {
        $salt = $this->secrets->derivedKey('scutoons ip salt ' . gmdate('Y-m-d', $this->clock->time()));
        return hash_hmac('sha256', $this->context->clientIp(), $salt);
    }

    private static function toon(?string $toon): ?string
    {
        return $toon !== null && preg_match('/^[a-z0-9_-]{1,64}$/', $toon) ? $toon : null;
    }

    /** First path segment of a redirect_to URL (https://scutoons.com/catpope → catpope) */
    public static function toonFromUrl(?string $url): ?string
    {
        $path = trim((string) parse_url((string) $url, PHP_URL_PATH), '/');
        return $path !== '' ? strtolower(explode('/', $path)[0]) : null;
    }
}
