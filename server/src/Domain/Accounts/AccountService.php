<?php
declare(strict_types=1);

namespace Scutoons\Domain\Accounts;

use Scutoons\Domain\Access\Permissions;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Avatar\AvatarService;
use Scutoons\Domain\Codes\CodeRules;
use Scutoons\Domain\Inventory\InventoryService;
use Scutoons\Domain\Ownership\OwnershipTransfers;
use Scutoons\Domain\Reader\ReaderState;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Time;

/** The signed-in reader's own account: GET /me/ and PUT /me/ */
final class AccountService
{
    public function __construct(
        private readonly Database $db,
        private readonly Clock $clock,
        private readonly Users $users,
        private readonly Permissions $permissions,
        private readonly InventoryService $inventory,
        private readonly AvatarService $avatar,
        private readonly ReaderState $reader,
        private readonly OwnershipTransfers $transfers,
    ) {
    }

    /**
     * GET /me/ → { user, access, memberships, inventory, avatar, stats, effects, received } (access incl. access_token when unlocked)
     * @param array<string, mixed> $user
     * @return array<string, mixed>
     */
    public function me(array $user): array
    {
        $userId = (string) $user['id'];
        $this->avatar->ensureWardrobe($userId); // default wearables for accounts older than them (agents/ITEMS.md "Wardrobe")
        $codes = $this->db->all('SELECT code, type, expires_at, revoked_at FROM qr WHERE claimed_by = ? ORDER BY claimed_at, code', [$userId]);
        // Owned codes of claimable types (permissions map); access = their grants while not expired
        $owned = array_values(array_filter($codes, fn(array $r) => $this->permissions->claimable((string) $r['type'])));
        return [
            'user' => $this->users->toJson($user),
            'access' => $this->permissions->access($codes, $userId, $this->signupCodeRow($user, $codes)),
            'memberships' => array_map(fn(array $r) => [
                'code' => $r['code'],
                'type' => $r['type'],
                'expires_at' => Time::localIso($r['expires_at']),
                'grants' => $this->permissions->grants((string) $r['type']),
            ], $owned),
            'inventory' => $this->inventory->entries($userId),
            'avatar' => $this->avatar->toJson($userId),
            // reader stats (decayed now) and running effects – agents/ITEMS.md "Substances"
            'stats' => (object) $this->reader->stats($userId),
            'effects' => $this->reader->effects($userId),
            // what arrived and was not seen yet: gifts and codes an admin moved here (agents/ITEMS.md "Ownership, claims,
            // gifting"); POST /inventory/received/seen/ marks them
            'received' => $this->transfers->unseen($userId),
        ];
    }

    /**
     * PUT /me/ { marketing_opt_in?: bool, hide_sensitive?: bool, show_hints?: bool, media_quality?: auto|low|medium|high, display_name?:
     * string|null } (at least one) → { user, avatar }. 400 invalid_body. `display_name` = the reader's name (askText,
     * `{reader.name}`): plain text, trimmed, at most 32 characters; null / "" clears it.
     * @param array<string, mixed> $user
     * @param array<string, mixed> $body
     * @return array{user: array<string, mixed>, avatar: array<string, mixed>}
     */
    public function update(array $user, array $body): array
    {
        $fields = array_intersect_key($body, array_flip(['marketing_opt_in', 'hide_sensitive', 'show_hints', 'media_quality', 'display_name']));
        $flags = array_diff_key($fields, ['media_quality' => true, 'display_name' => true]);
        $name = array_key_exists('display_name', $fields) ? Users::displayName($fields['display_name']) : null;
        if ($fields === [] || array_filter($flags, fn($v) => !is_bool($v)) !== []
            || (array_key_exists('media_quality', $fields) && !in_array($fields['media_quality'], Users::MEDIA_QUALITIES, true))) {
            throw new ApiError(400, 'invalid_body', 'marketing_opt_in and/or hide_sensitive and/or show_hints (true/false) and/or media_quality (auto, low, medium, high) and/or display_name are required.');
        }
        if ($name === false) throw new ApiError(400, 'invalid_body', 'display_name is a text of at most ' . Users::DISPLAY_NAME_MAX . ' characters (or null).');
        $userId = (string) $user['id'];
        if (array_key_exists('display_name', $fields)) $this->users->setDisplayName($userId, $name);
        if (array_key_exists('media_quality', $fields)) $this->users->setMediaQuality($userId, (string) $fields['media_quality']);
        if (array_key_exists('marketing_opt_in', $fields)) $this->users->setMarketingOptIn($userId, $fields['marketing_opt_in']);
        if (array_key_exists('hide_sensitive', $fields)) $this->users->setHideSensitive($userId, $fields['hide_sensitive']);
        if (array_key_exists('show_hints', $fields)) $this->users->setShowHints($userId, $fields['show_hints']);
        return ['user' => $this->users->toJson($this->users->byId($userId) ?? $user), 'avatar' => $this->avatar->toJson($userId)];
    }

    /**
     * The code the account was created with, when it is not owned by the account (a promo code): its grants count for
     * /me access while it exists, is not expired or revoked and its type grants something – so a promo account reads
     * on every device. It never becomes an owned code (no inventory, not in memberships).
     * @param array<string, mixed> $user
     * @param list<array<string, mixed>> $owned the owned qr rows
     * @return array<string, mixed>|null
     */
    private function signupCodeRow(array $user, array $owned): ?array
    {
        $code = $user['signup_code'] ?? null;
        if (!is_string($code) || $code === '' || in_array($code, array_column($owned, 'code'), true)) return null;
        $row = $this->db->one('SELECT code, type, expires_at, revoked_at FROM qr WHERE code = ?', [$code]);
        if (!$row || CodeRules::isExpired($row, $this->clock->time()) || CodeRules::isRevoked($row) || !$this->permissions->grants((string) $row['type'])) return null;
        return $row;
    }
}
