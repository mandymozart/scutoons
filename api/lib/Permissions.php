<?php
declare(strict_types=1);

/**
 * What a code type unlocks – the permissions map `api/permissions.json` (ITEMS.md "Access model"), edited by the
 * owner, no code change needed. Every claimable/grants decision goes through this class:
 *   types.<type>.claimable  can be owned (one owner, goes into the inventory, /auth/code/ works)
 *   types.<type>.grants     permission ids the owner has while the code is not expired (e.g. "catalog")
 *   default                 for types not listed
 * Another file can be used with the config key PERMISSIONS_PATH (tests, staging).
 */
final class Permissions
{
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
     * Access from the codes a user owns (qr rows with code, type, expires_at).
     * @param list<array<string, mixed>> $rows
     * @return array{permissions: string[], unlocked: bool, expires_at: ?string, codes: string[]}
     */
    public static function access(array $rows): array
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
        $expiresAt = null;
        if ($expiries && !in_array(null, $expiries, true)) $expiresAt = date('c', max($expiries));
        return [
            'permissions' => $permissions,
            'unlocked' => in_array('catalog', $permissions, true),
            'expires_at' => $expiresAt,
            'codes' => $codes,
        ];
    }
}
