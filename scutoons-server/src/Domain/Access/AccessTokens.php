<?php
declare(strict_types=1);

namespace Scutoons\Domain\Access;

use Scutoons\Domain\Secrets;
use Scutoons\Infrastructure\Clock\Clock;

/**
 * Signed access tokens of /access/ and /me/ (`access_token`): base64url(json payload) "." base64url(HMAC-SHA256),
 * payload { v:1, perms, exp, sub }, exp = min(code expiry, now + 30 days), sub = "code:<keyed hash>" | "user:<id>".
 * Opaque to the viewer; verify() is ready for a later server-side content gate.
 */
final class AccessTokens
{
    public const MAX_SECONDS = 30 * 86400;

    public function __construct(private readonly Secrets $secrets, private readonly Clock $clock)
    {
    }

    /** `sub` of a device token: a keyed hash of the code (the token never reveals the code itself) */
    public function codeSubject(string $code): string
    {
        return 'code:' . substr(hash_hmac('sha256', $code, $this->key()), 0, 32);
    }

    /** @param list<string> $permissions */
    public function issue(array $permissions, ?int $codeExpires, string $subject, ?int $now = null): string
    {
        $now ??= $this->clock->time();
        $exp = $now + self::MAX_SECONDS;
        if ($codeExpires !== null) $exp = min($exp, $codeExpires);
        $payload = self::b64((string) json_encode(['v' => 1, 'perms' => array_values($permissions), 'exp' => $exp, 'sub' => $subject], JSON_UNESCAPED_SLASHES));
        return $payload . '.' . self::b64(hash_hmac('sha256', $payload, $this->key(), true));
    }

    /** @return array<string, mixed>|null the payload of a valid, unexpired token; null otherwise */
    public function verify(string $token, ?int $now = null): ?array
    {
        if (!preg_match('/^([A-Za-z0-9_-]+)\.([A-Za-z0-9_-]+)$/', $token, $m)) return null;
        $expected = self::b64(hash_hmac('sha256', $m[1], $this->key(), true));
        if (!hash_equals($expected, $m[2])) return null;
        $payload = json_decode((string) base64_decode(strtr($m[1], '-_', '+/'), true), true);
        if (!is_array($payload) || ($payload['v'] ?? null) !== 1 || !is_array($payload['perms'] ?? null)
            || !is_int($payload['exp'] ?? null) || !is_string($payload['sub'] ?? null)) return null;
        if ($payload['exp'] <= ($now ?? $this->clock->time())) return null;
        return $payload;
    }

    /** Key for the access tokens, derived from AUTH_SECRET with its own label (not the login-code key) */
    private function key(): string
    {
        return $this->secrets->derivedKey('scutoons access token v1');
    }

    private static function b64(string $bytes): string
    {
        return rtrim(strtr(base64_encode($bytes), '+/', '-_'), '=');
    }
}
