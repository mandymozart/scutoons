<?php
declare(strict_types=1);

namespace Scutoons\Domain\Access;

use RuntimeException;
use Scutoons\Domain\Catalog\CatalogStore;
use Scutoons\Domain\Codes\CodeRules;
use Scutoons\Infrastructure\Clock\Clock;

/**
 * What a code type grants – the permissions map (agents/ITEMS.md "Access model"): the tables catalog_permissions +
 * catalog_code_types (agents/CATALOG.md), edited by admins (/api/admin/catalog/…), no
 * code change needed. Server-only: never served to readers. Every claimable/signup/grants decision goes through this
 * class:
 *   types.<type>.claimable  can be owned by one account (inventory, edition number)
 *   types.<type>.signup     a new account can be created with a code of this type (membership, promo)
 *   types.<type>.grants     permission ids while the code is valid – for anyone holding the code (device access via
 *                           /access/) and for its owner (/me/)
 *   default                 for types not listed
 * A map file instead of the tables: useFile() (tests).
 */
final class Permissions
{
    /** @var array<string, mixed>|null */
    private ?array $map = null;
    private ?string $override = null;

    public function __construct(
        private readonly AccessTokens $tokens,
        private readonly Clock $clock,
        private readonly CatalogStore $store,
    ) {
    }

    /** Use a map file instead of the tables (null = back to the tables) – for tests */
    public function useFile(?string $path): void
    {
        $this->override = $path;
        $this->map = null;
    }

    /**
     * The map in the shape of the old permissions.json: { version, permissions: { id: description }, types: { type:
     * { claimable, signup, grants } }, default } – from the tables (or the useFile() file)
     * @return array<string, mixed>
     */
    public function map(): array
    {
        if ($this->map !== null) return $this->map;
        if ($this->override !== null) {
            $map = json_decode((string) @file_get_contents($this->override), true);
            if (!is_array($map) || !is_array($map['types'] ?? null)) {
                throw new RuntimeException('permissions map is missing or invalid: ' . $this->override);
            }
            return $this->map = $map;
        }
        $types = $this->store->map('code-types');
        $default = $types[CatalogStore::DEFAULT_CODE_TYPE] ?? null;
        unset($types[CatalogStore::DEFAULT_CODE_TYPE]);
        $meta = $this->store->get('meta', 'permissions');
        $map = [
            'version' => is_array($meta) ? ($meta['version'] ?? 1) : 1,
            'permissions' => $this->store->map('permissions'),
            'types' => $types,
        ];
        if ($default !== null) $map['default'] = $default;
        return $map;
    }

    /** @return array{claimable: bool, signup: bool, grants: list<string>} */
    public function forType(string $type): array
    {
        $map = $this->map();
        $entry = $map['types'][$type] ?? $map['default'] ?? [];
        return [
            'claimable' => (bool) ($entry['claimable'] ?? false),
            'signup' => (bool) ($entry['signup'] ?? false),
            'grants' => array_values(array_filter((array) ($entry['grants'] ?? []), 'is_string')),
        ];
    }

    public function claimable(string $type): bool
    {
        return $this->forType($type)['claimable'];
    }

    /** A new account may be created with a code of this type */
    public function signup(string $type): bool
    {
        return $this->forType($type)['signup'];
    }

    /** @return list<string> */
    public function grants(string $type): array
    {
        return $this->forType($type)['grants'];
    }

    /**
     * Account access (/me/ `access`) from the codes a user owns (qr rows with code, type, expires_at, revoked_at)
     * plus, optionally, the still valid signup code the account was created with (a promo code; not owned).
     * @param list<array<string, mixed>> $rows
     * @param array<string, mixed>|null $signupRow
     * @return array<string, mixed> { permissions, unlocked, expires_at, codes, signup_code, access_token? }
     */
    public function access(array $rows, string $userId, ?array $signupRow = null): array
    {
        if ($signupRow !== null) $rows[] = $signupRow;
        $permissions = [];
        $codes = [];
        $expiries = [];
        $now = $this->clock->time();
        foreach ($rows as $row) {
            if (CodeRules::isExpired($row, $now) || CodeRules::isRevoked($row)) continue;
            $grants = $this->grants((string) $row['type']);
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
            'signup_code' => $signupRow !== null && in_array($signupRow['code'], $codes, true) ? $signupRow['code'] : null,
        ];
        if ($access['unlocked']) $access['access_token'] = $this->tokens->issue($permissions, $expires, 'user:' . $userId);
        return $access;
    }
}
