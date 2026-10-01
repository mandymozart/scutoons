<?php
declare(strict_types=1);

namespace Scutoons\Domain\Accounts;

use Scutoons\Domain\Access\Permissions;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Avatar\AvatarService;
use Scutoons\Domain\Codes\CodeRules;
use Scutoons\Domain\Inventory\InventoryService;
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
    ) {
    }

    /**
     * GET /me/ → { user, access, memberships, inventory, avatar } (access incl. access_token when unlocked)
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
        ];
    }

    /**
     * PUT /me/ { marketing_opt_in?: bool, hide_sensitive?: bool } (at least one) → { user, avatar }. Turning
     * hide_sensitive on while wearing a sensitive A-pose look moves the body to the same character's first
     * non-sensitive look (AvatarService::hideSensitiveBody). 400 invalid_body.
     * @param array<string, mixed> $user
     * @param array<string, mixed> $body
     * @return array{user: array<string, mixed>, avatar: array<string, mixed>}
     */
    public function update(array $user, array $body): array
    {
        $fields = array_intersect_key($body, array_flip(['marketing_opt_in', 'hide_sensitive']));
        if ($fields === [] || array_filter($fields, fn($v) => !is_bool($v)) !== []) {
            throw new ApiError(400, 'invalid_body', 'marketing_opt_in and/or hide_sensitive (true/false) are required.');
        }
        $userId = (string) $user['id'];
        if (array_key_exists('marketing_opt_in', $fields)) $this->users->setMarketingOptIn($userId, $fields['marketing_opt_in']);
        if (array_key_exists('hide_sensitive', $fields)) {
            $this->users->setHideSensitive($userId, $fields['hide_sensitive']);
            // (own transaction) a worn sensitive body moves to the character's first non-sensitive look
            if ($fields['hide_sensitive']) {
                $this->avatar->hideSensitiveBody($userId);
                $this->avatar->coverSensitive($userId); // empty legs get the swim pants back (agents/ITEMS.md "Wardrobe")
            }
        }
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
