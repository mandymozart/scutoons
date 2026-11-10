<?php
declare(strict_types=1);

namespace Scutoons\Domain\Live;

use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;

/**
 * A live session (LIVE.md B.6): a code, and tokens the relay checks with the secret it shares with this API
 * (`LIVE_TOKEN_SECRET`; deck/relay/src/token.rs is the other half: `base64url(payload JSON) "." base64url(HMAC-SHA256)`, tested
 * against one vector on both sides). Unset secret: the relay's dev mode, any token passes, so `dev` is one.
 */
final class LiveSessions
{
    public const HOURS = 24;
    private const ALPHABET = 'ABCDEFGHJKLMNPQRSTUVWXYZ23456789';

    public function __construct(private readonly Settings $settings, private readonly Clock $clock)
    {
    }

    /** @return array{code: string, token: string, tokens: array{bridge: string, seat: string, uplink: string}, dev: bool} */
    public function open(?string $show): array
    {
        $code = '';
        for ($i = 0; $i < 6; $i++) $code .= self::ALPHABET[random_int(0, strlen(self::ALPHABET) - 1)];
        $secret = $this->settings->get('LIVE_TOKEN_SECRET');
        $exp = $this->clock->time() + self::HOURS * 3600;
        $token = fn(string $role): string => $secret === '' ? 'dev' : self::sign($secret, ['session' => $code, 'role' => $role, 'exp' => $exp] + ($show === null ? [] : ['show' => $show]));
        return ['code' => $code, 'token' => $token('conductor'), 'tokens' => ['bridge' => $token('bridge'), 'seat' => $token('seat'), 'uplink' => $token('uplink')], 'dev' => $secret === ''];
    }

    /** @param array<string, mixed> $claims in the order the relay writes them */
    public static function sign(string $secret, array $claims): string
    {
        $b64 = fn(string $bytes): string => rtrim(strtr(base64_encode($bytes), '+/', '-_'), '=');
        $payload = $b64((string) json_encode($claims, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE));
        return $payload . '.' . $b64(hash_hmac('sha256', $payload, $secret, true));
    }
}
