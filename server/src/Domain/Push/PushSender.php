<?php
declare(strict_types=1);

namespace Scutoons\Domain\Push;

use RuntimeException;
use Scutoons\Domain\ApiError;
use Scutoons\Infrastructure\AssetStorage\CurlHttpClient;
use Scutoons\Infrastructure\AssetStorage\HttpClient;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;

/**
 * Sends one notification per subscription (agents/research/PWA.md §2b): the payload encrypted for that browser
 * (WebPushCrypto, `aes128gcm`), VAPID from the settings VAPID_PUBLIC_KEY / VAPID_PRIVATE_KEY / VAPID_SUBJECT (env /
 * the deploy's config.php – never in git). 201 = delivered to the push service; 404 / 410 = the subscription is gone
 * (deleted); anything else counts as a failure (dropped after PushSubscriptions::MAX_FAILURES in a row).
 */
final class PushSender
{
    /** how long a push service keeps an undelivered message (a phone that is off): one day */
    private const TTL = 86400;

    private HttpClient $http;

    public function __construct(private readonly Settings $settings, private readonly PushSubscriptions $subscriptions, private readonly Clock $clock)
    {
        $this->http = new CurlHttpClient(15);
    }

    /** tests: a fake push service */
    public function useHttp(HttpClient $http): void
    {
        $this->http = $http;
    }

    public function configured(): bool
    {
        return $this->publicKey() !== '' && WebPushCrypto::validKeys($this->publicKey(), $this->settings->get('VAPID_PRIVATE_KEY'));
    }

    /** the key browsers subscribe with (applicationServerKey), '' = push is off on this server */
    public function publicKey(): string
    {
        return trim($this->settings->get('VAPID_PUBLIC_KEY'));
    }

    /**
     * @param list<array{id: string, endpoint: string, p256dh: string, auth: string, locale: string}> $recipients
     * @param callable(string): array<string, mixed> $payload the notification for a locale (title, body, url, tag)
     * @return array{recipients: int, delivered: int, gone: int, failed: int}
     */
    public function send(array $recipients, callable $payload): array
    {
        if (!$this->configured()) throw new ApiError(409, 'push_not_configured', 'Web Push has no VAPID keys on this server (VAPID_PUBLIC_KEY / VAPID_PRIVATE_KEY).');
        $result = ['recipients' => count($recipients), 'delivered' => 0, 'gone' => 0, 'failed' => 0];
        $bodies = [];
        $public = $this->publicKey();
        $private = trim($this->settings->get('VAPID_PRIVATE_KEY'));
        $subject = trim($this->settings->get('VAPID_SUBJECT')) ?: 'mailto:hello@scutoons.com';
        // a JWT may live up to 24 h; 12 h is plenty for one send
        $expires = $this->clock->time() + 12 * 3600;
        @set_time_limit(max(60, count($recipients) * 2));
        foreach ($recipients as $to) {
            $bodies[$to['locale']] ??= (string) json_encode($payload($to['locale']), JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
            try {
                $response = $this->http->send('POST', $to['endpoint'], [
                    'Authorization' => WebPushCrypto::vapidHeader($to['endpoint'], $public, $private, $subject, $expires),
                    'Content-Encoding' => 'aes128gcm',
                    'Content-Type' => 'application/octet-stream',
                    'TTL' => (string) self::TTL,
                    'Urgency' => 'normal',
                ], WebPushCrypto::encrypt($bodies[$to['locale']], $to['p256dh'], $to['auth']));
                $status = $response['status'];
            } catch (RuntimeException) {
                $status = 0;
            }
            if ($status >= 200 && $status < 300) {
                $result['delivered']++;
                $this->subscriptions->delivered($to['id']);
            } elseif ($status === 404 || $status === 410) {
                $result['gone']++;
                $this->subscriptions->gone($to['id']);
            } else {
                $result['failed']++;
                $this->subscriptions->failed($to['id']);
            }
        }
        return $result;
    }
}
