<?php
declare(strict_types=1);

namespace Scutoons\Domain\Push;

use Scutoons\Domain\ApiError;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Database\Database;

/**
 * The browsers that opted in to Web Push on the account page (table push_subscriptions, agents/research/PWA.md §2b).
 * A subscription is keyed by its endpoint; the signed-in account is remembered (a test send goes to the admin's own
 * browsers; deleting the account deletes them). Endpoints must be https on a known push service – the server POSTs to
 * them, so nothing else (no internal hosts) is accepted. `app` = the app it subscribed in: the reader's "new chapter"
 * goes to `viewer` only, a team kind only to its tool (research/PWA-APPS.md §5).
 */
final class PushSubscriptions
{
    /** the push services of Chrome / Edge / Firefox / Safari (host suffixes) */
    private const SERVICES = ['fcm.googleapis.com', 'android.googleapis.com', 'push.services.mozilla.com', 'push.apple.com', 'notify.windows.com'];
    public const LOCALES = ['de', 'en'];
    /** the apps with push: the reader and the three tools */
    public const APPS = ['viewer', 'creator', 'admin', 'embassy'];
    /** refused sends in a row (other than 404 / 410) before a subscription is dropped */
    public const MAX_FAILURES = 5;

    public function __construct(private readonly Database $db, private readonly Clock $clock)
    {
    }

    /**
     * POST /api/push/subscribe { subscription: { endpoint, keys: { p256dh, auth } }, locale, app? (default viewer) }
     * @param array<string, mixed> $body
     * @param array<string, mixed>|null $user
     */
    public function save(array $body, ?array $user): void
    {
        $subscription = is_array($body['subscription'] ?? null) ? $body['subscription'] : $body;
        $endpoint = self::endpoint($subscription['endpoint'] ?? null);
        $keys = is_array($subscription['keys'] ?? null) ? $subscription['keys'] : [];
        $p256dh = (string) ($keys['p256dh'] ?? '');
        $auth = (string) ($keys['auth'] ?? '');
        try {
            $validKeys = strlen(WebPushCrypto::unb64($p256dh)) === 65 && strlen(WebPushCrypto::unb64($auth)) === 16;
        } catch (\RuntimeException) {
            $validKeys = false;
        }
        if (!$validKeys) throw new ApiError(400, 'invalid_subscription', 'The subscription keys are missing or malformed.');
        $locale = in_array($body['locale'] ?? null, self::LOCALES, true) ? (string) $body['locale'] : self::LOCALES[0];
        $app = $body['app'] ?? 'viewer';
        if (!in_array($app, self::APPS, true)) throw new ApiError(400, 'invalid_app', 'Unknown app.');
        $now = $this->clock->utc();
        $this->db->run(
            'INSERT INTO push_subscriptions (id, endpoint, p256dh, auth, user_id, locale, app, failures, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, 0, ?, ?)
             ON DUPLICATE KEY UPDATE p256dh = VALUES(p256dh), auth = VALUES(auth), user_id = COALESCE(VALUES(user_id), user_id),
               locale = VALUES(locale), app = VALUES(app), failures = 0, updated_at = VALUES(updated_at)',
            [hash('sha256', $endpoint), $endpoint, $p256dh, $auth, $user['id'] ?? null, $locale, $app, $now, $now]
        );
    }

    /** POST /api/push/unsubscribe { endpoint } – whoever knows the endpoint (only that browser does) */
    public function remove(mixed $endpoint): void
    {
        if (!is_string($endpoint) || $endpoint === '') throw new ApiError(400, 'invalid_subscription', 'No endpoint.');
        $this->db->run('DELETE FROM push_subscriptions WHERE id = ?', [hash('sha256', $endpoint)]);
    }

    /** @return array{total: int, locales: array<string, int>, apps: array<string, int>} the reader's per locale, every app's total */
    public function counts(): array
    {
        $locales = [];
        foreach ($this->db->all("SELECT locale, COUNT(*) AS n FROM push_subscriptions WHERE app = 'viewer' GROUP BY locale") as $row) $locales[(string) $row['locale']] = (int) $row['n'];
        $apps = [];
        foreach ($this->db->all('SELECT app, COUNT(*) AS n FROM push_subscriptions GROUP BY app') as $row) $apps[(string) $row['app']] = (int) $row['n'];
        return ['total' => array_sum($locales), 'locales' => $locales, 'apps' => $apps];
    }

    /** a browser of this account is subscribed in this app (the tools' settings page) */
    public function has(string $userId, string $app): bool
    {
        return (bool) $this->db->value('SELECT 1 FROM push_subscriptions WHERE user_id = ? AND app = ? LIMIT 1', [$userId, $app]);
    }

    /**
     * The recipients in one app – everyone, or only one account's browsers (a test send, a team kind)
     * @return list<array{id: string, endpoint: string, p256dh: string, auth: string, locale: string}>
     */
    public function recipients(?string $userId = null, string $app = 'viewer'): array
    {
        $rows = $userId === null
            ? $this->db->all('SELECT id, endpoint, p256dh, auth, locale FROM push_subscriptions WHERE app = ? ORDER BY created_at', [$app])
            : $this->db->all('SELECT id, endpoint, p256dh, auth, locale FROM push_subscriptions WHERE user_id = ? AND app = ? ORDER BY created_at', [$userId, $app]);
        return array_map(static fn(array $r): array => [
            'id' => (string) $r['id'], 'endpoint' => (string) $r['endpoint'], 'p256dh' => (string) $r['p256dh'], 'auth' => (string) $r['auth'], 'locale' => (string) $r['locale'],
        ], $rows);
    }

    /** the push service said the subscription is gone (404 / 410) */
    public function gone(string $id): void
    {
        $this->db->run('DELETE FROM push_subscriptions WHERE id = ?', [$id]);
    }

    /** another refusal; dropped after MAX_FAILURES in a row */
    public function failed(string $id): void
    {
        $this->db->run('UPDATE push_subscriptions SET failures = failures + 1 WHERE id = ?', [$id]);
        $this->db->run('DELETE FROM push_subscriptions WHERE id = ? AND failures >= ?', [$id, self::MAX_FAILURES]);
    }

    public function delivered(string $id): void
    {
        $this->db->run('UPDATE push_subscriptions SET failures = 0 WHERE id = ? AND failures > 0', [$id]);
    }

    /** an https URL on a known push service, else 400 */
    public static function endpoint(mixed $value): string
    {
        $endpoint = is_string($value) ? trim($value) : '';
        $parts = $endpoint !== '' && strlen($endpoint) <= 1024 ? parse_url($endpoint) : false;
        $host = is_array($parts) && ($parts['scheme'] ?? '') === 'https' && !isset($parts['user']) && !isset($parts['port']) ? strtolower((string) ($parts['host'] ?? '')) : '';
        foreach (self::SERVICES as $service) {
            if ($host === $service || str_ends_with($host, '.' . $service)) return $endpoint;
        }
        throw new ApiError(400, 'invalid_subscription', 'Not a push service endpoint.');
    }
}
