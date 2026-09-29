<?php
declare(strict_types=1);

/**
 * What a code type grants – the permissions map `api/permissions.json` (ITEMS.md "Access model"), edited by the
 * owner, no code change needed. Server-only: the file is not served over HTTP. Every claimable/grants decision goes
 * through this class:
 *   types.<type>.claimable  can be owned by one account (inventory, edition number)
 *   types.<type>.grants     permission ids while the code is not expired – for anyone holding the code (device
 *                           access via /access/) and for its owner (/me/)
 *   default                 for types not listed
 * Another file can be used with the config key PERMISSIONS_PATH (tests, staging) or Permissions::useFile().
 *
 * Access tokens: /access/ and /me/ also return a signed `access_token` (HMAC-SHA256, compact `payload.signature`,
 * base64url) carrying { v:1, perms, exp, sub } for a later server-side content gate (verifyToken()).
 */
final class Permissions
{
    public const TOKEN_MAX_SECONDS = 30 * 86400;

    private static ?array $map = null;
    private static ?string $override = null;

    public static function path(): string
    {
        return self::$override ?? (Config::get('PERMISSIONS_PATH') ?: __DIR__ . '/../permissions.json');
    }

    /** Use another map file (null = back to the configured one) – for tests */
    public static function useFile(?string $path): void
    {
        self::$override = $path;
        self::$map = null;
    }

    /** @return array<string, mixed> */
    public static function map(): array
    {
        if (self::$map === null) {
            $map = json_decode((string) @file_get_contents(self::path()), true);
            if (!is_array($map) || !is_array($map['types'] ?? null)) throw new RuntimeException('permissions.json is missing or invalid: ' . self::path());
            self::$map = $map;
        }
        return self::$map;
    }

    /** @return array{claimable: bool, grants: string[]} */
    public static function forType(string $type): array
    {
        $map = self::map();
        $entry = $map['types'][$type] ?? $map['default'] ?? [];
        return [
            'claimable' => (bool) ($entry['claimable'] ?? false),
            'grants' => array_values(array_filter((array) ($entry['grants'] ?? []), 'is_string')),
        ];
    }

    public static function claimable(string $type): bool
    {
        return self::forType($type)['claimable'];
    }

    /** @return string[] */
    public static function grants(string $type): array
    {
        return self::forType($type)['grants'];
    }

    /**
     * Account access from the codes a user owns (qr rows with code, type, expires_at) – for /me/.
     * @param list<array<string, mixed>> $rows
     * @return array<string, mixed> { permissions, unlocked, expires_at, codes, access_token? }
     */
    public static function access(array $rows, string $userId): array
    {
        $permissions = [];
        $codes = [];
        $expiries = [];
        foreach ($rows as $row) {
            if (qrIsExpired($row)) continue;
            $grants = self::grants((string) $row['type']);
            if (!$grants) continue;
            $permissions = array_merge($permissions, $grants);
            $codes[] = $row['code'];
            $expiries[] = $row['expires_at'] ? (int) strtotime((string) $row['expires_at']) : null;
        }
        $permissions = array_values(array_unique($permissions));
        // Access lasts as long as the longest-running code; a code without expiry = no end
        $expires = ($expiries && !in_array(null, $expiries, true)) ? max($expiries) : null;
        $access = [
            'permissions' => $permissions,
            'unlocked' => in_array('catalog', $permissions, true),
            'expires_at' => $expires !== null ? date('c', $expires) : null,
            'codes' => $codes,
        ];
        if ($access['unlocked']) $access['access_token'] = self::issueToken($permissions, $expires, 'user:' . $userId);
        return $access;
    }

    /** `sub` of a device token: a keyed hash of the code (the token never reveals the code itself) */
    public static function codeSubject(string $code): string
    {
        return 'code:' . substr(hash_hmac('sha256', $code, self::tokenKey()), 0, 32);
    }

    /**
     * Signed access token: base64url(json payload) . '.' . base64url(HMAC-SHA256(payload part, key)).
     * exp = min(code expiry, now + 30 days).
     * @param string[] $permissions
     */
    public static function issueToken(array $permissions, ?int $codeExpires, string $subject, ?int $now = null): string
    {
        $now ??= time();
        $exp = $now + self::TOKEN_MAX_SECONDS;
        if ($codeExpires !== null) $exp = min($exp, $codeExpires);
        $payload = self::b64(json_encode(['v' => 1, 'perms' => array_values($permissions), 'exp' => $exp, 'sub' => $subject], JSON_UNESCAPED_SLASHES));
        return $payload . '.' . self::b64(hash_hmac('sha256', $payload, self::tokenKey(), true));
    }

    /** @return array<string, mixed>|null the payload of a valid, unexpired token; null otherwise */
    public static function verifyToken(string $token, ?int $now = null): ?array
    {
        if (!preg_match('/^([A-Za-z0-9_-]+)\.([A-Za-z0-9_-]+)$/', $token, $m)) return null;
        $expected = self::b64(hash_hmac('sha256', $m[1], self::tokenKey(), true));
        if (!hash_equals($expected, $m[2])) return null;
        $payload = json_decode((string) base64_decode(strtr($m[1], '-_', '+/'), true), true);
        if (!is_array($payload) || ($payload['v'] ?? null) !== 1 || !is_array($payload['perms'] ?? null)
            || !is_int($payload['exp'] ?? null) || !is_string($payload['sub'] ?? null)) return null;
        if ($payload['exp'] <= ($now ?? time())) return null;
        return $payload;
    }

    /** Key for the access tokens, derived from AUTH_SECRET with its own label (not the login-code key) */
    private static function tokenKey(): string
    {
        return hash_hmac('sha256', 'scutoons access token v1', Auth::secret(), true);
    }

    private static function b64(string $bytes): string
    {
        return rtrim(strtr(base64_encode($bytes), '+/', '-_'), '=');
    }
}
