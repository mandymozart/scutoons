<?php
declare(strict_types=1);

/**
 * Activity statistics per code (table code_events, append-only; ITEMS.md "Revoke + activity statistics").
 * Events: scan (landing page /?c=), access (/access/, at most once per code + device per UTC day), claim,
 * claim_refused, login (one row per owned code at sign-in), revoke, unrevoke.
 *
 * Privacy: no raw IP, email or user agent. device_hash = HMAC of the browser's random `X-Device-Id` header;
 * ip_hash = HMAC of the IP with a daily salt derived from AUTH_SECRET + the UTC date (no salt table; hashes of
 * different days can't be linked). Logging never breaks a request: failures only go to the error log.
 */
final class Events
{
    public const TYPES = ['scan', 'access', 'claim', 'claim_refused', 'login', 'revoke', 'unrevoke'];

    public static function log(string $event, string $code, ?string $userId = null, ?string $toon = null): void
    {
        try {
            if (!in_array($event, self::TYPES, true)) throw new InvalidArgumentException("unknown event $event");
            Db::run('INSERT INTO code_events (code, event, device_hash, user_id, ip_hash, toon, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)',
                [substr($code, 0, 8), $event, self::deviceHash(), $userId, self::ipHash(), self::toon($toon), Db::now()]);
        } catch (Throwable $error) {
            error_log('[qr-api] event log failed: ' . $error->getMessage());
        }
    }

    /** `access` once per code + device (or, without a device id, per daily IP hash) per UTC day */
    public static function logAccess(string $code, ?string $userId = null, ?string $toon = null): void
    {
        try {
            $device = self::deviceHash();
            $dayStart = gmdate('Y-m-d') . ' 00:00:00';
            $seen = $device !== null
                ? Db::value("SELECT 1 FROM code_events WHERE code = ? AND event = 'access' AND device_hash = ? AND created_at >= ? LIMIT 1", [$code, $device, $dayStart])
                : Db::value("SELECT 1 FROM code_events WHERE code = ? AND event = 'access' AND device_hash IS NULL AND ip_hash = ? AND created_at >= ? LIMIT 1", [$code, self::ipHash(), $dayStart]);
            if ($seen === null) self::log('access', $code, $userId, $toon);
        } catch (Throwable $error) {
            error_log('[qr-api] access log failed: ' . $error->getMessage());
        }
    }

    /** HMAC of the X-Device-Id header (random per browser, localStorage `scutoons_device`), or null */
    public static function deviceHash(): ?string
    {
        $id = (string) ($_SERVER['HTTP_X_DEVICE_ID'] ?? '');
        if (!preg_match('/^[A-Za-z0-9_-]{8,128}$/', $id)) return null;
        return hash_hmac('sha256', $id, hash_hmac('sha256', 'scutoons device id v1', Auth::secret(), true));
    }

    /** HMAC of the client IP with today's salt (AUTH_SECRET + UTC date) */
    public static function ipHash(): string
    {
        $salt = hash_hmac('sha256', 'scutoons ip salt ' . gmdate('Y-m-d'), Auth::secret(), true);
        return hash_hmac('sha256', Http::clientIp(), $salt);
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
