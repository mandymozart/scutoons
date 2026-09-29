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
     * PUT /me/ { marketing_opt_in: bool } → the updated user
     * @param array<string, mixed> $user
     * @param array<string, mixed> $body
     * @return array<string, mixed>
     */
    public function update(array $user, array $body): array
    {
        if (!array_key_exists('marketing_opt_in', $body) || !is_bool($body['marketing_opt_in'])) {
            throw new ApiError(400, 'invalid_body', 'marketing_opt_in (true/false) is required.');
        }
        $this->users->setMarketingOptIn((string) $user['id'], $body['marketing_opt_in']);
        return $this->users->toJson($this->users->byId((string) $user['id']) ?? $user);
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
