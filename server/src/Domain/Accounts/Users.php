<?php
declare(strict_types=1);

namespace Scutoons\Domain\Accounts;

use RuntimeException;
use Scutoons\Domain\Avatar\AvatarService;
use Scutoons\Domain\Catalog\Catalog;
use Scutoons\Domain\Inventory\InventoryService;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Ids;
use Scutoons\Support\Time;

/**
 * Reader accounts (email only). A new account needs a signup code (permissions.json "signup": true – a SCUTOONS
 * product or a promo code); the login service checks that before calling create().
 */
final class Users
{
    public function __construct(
        private readonly Database $db,
        private readonly Clock $clock,
        private readonly Roles $roles,
        private readonly InventoryService $inventory,
        private readonly Catalog $catalog,
        private readonly AvatarService $avatar,
    ) {
    }

    /** @return array<string, mixed>|null */
    public function byEmail(string $email): ?array
    {
        return $this->db->one('SELECT * FROM users WHERE email = ?', [$email]);
    }

    /** @return array<string, mixed>|null */
    public function byId(string $id): ?array
    {
        return $this->db->one('SELECT * FROM users WHERE id = ?', [$id]);
    }

    /**
     * Creates the user (after a confirmed login) with the `member` role and the starter items. Call inside a
     * transaction. $signupCode = the code the account was created with (users.signup_code).
     * @return array<string, mixed>
     */
    public function create(string $email, string $locale, ?string $signupCode = null, bool $marketingOptIn = false): array
    {
        $id = Ids::uuid();
        $now = $this->clock->utc();
        $this->db->run('INSERT INTO users (id, email, locale, created_at, confirmed_at, updated_at, signup_code, marketing_opt_in, marketing_opt_in_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)',
            [$id, $email, $locale, $now, $now, $now, $signupCode, $marketingOptIn ? 1 : 0, $marketingOptIn ? $now : null]);
        $this->roles->grant($id, Roles::MEMBER, 'signup');
        foreach ($this->catalog->starters() as $item) {
            if (is_string($item['slot'] ?? null)) continue; // the default wearables: granted below, the basic dress put on
            $this->inventory->add($id, (string) $item['id'], null, 1, 'starter');
        }
        $this->avatar->wearBasicDress($id, $this->inventory->grantStarterWearables($id));
        return $this->byId($id) ?? throw new RuntimeException('user not stored');
    }

    /** Sets "Send me news about new releases"; marketing_opt_in_at = time of the last change */
    public function setMarketingOptIn(string $userId, bool $optIn): void
    {
        $now = $this->clock->utc();
        $this->db->run('UPDATE users SET marketing_opt_in = ?, marketing_opt_in_at = ?, updated_at = ? WHERE id = ? AND marketing_opt_in <> ?',
            [$optIn ? 1 : 0, $now, $now, $userId, $optIn ? 1 : 0]);
    }

    /**
     * Sets "Hide sensitive content" (agents/ITEMS.md "Sensitive content"; on by default). Returns whether it changed.
     */
    public function setHideSensitive(string $userId, bool $hide): bool
    {
        return $this->db->run('UPDATE users SET hide_sensitive = ?, updated_at = ? WHERE id = ? AND hide_sensitive <> ?',
            [$hide ? 1 : 0, $this->clock->utc(), $userId, $hide ? 1 : 0])->rowCount() > 0;
    }

    /** "Media quality" values (agents/DESIGN.md "Media quality"): auto = by the reader's bandwidth (the default) */
    public const MEDIA_QUALITIES = ['auto', 'low', 'medium', 'high'];
    public const MEDIA_QUALITY_DEFAULT = 'auto';

    /** Sets "Media quality" (one of MEDIA_QUALITIES). Returns whether it changed. */
    public function setMediaQuality(string $userId, string $quality): bool
    {
        return $this->db->run('UPDATE users SET media_quality = ?, updated_at = ? WHERE id = ? AND media_quality <> ?',
            [$quality, $this->clock->utc(), $userId, $quality])->rowCount() > 0;
    }

    /**
     * "Media quality" of a users row (auto when the column is missing – not migrated yet – or holds something unknown)
     * @param array<string, mixed> $row
     */
    public static function mediaQuality(array $row): string
    {
        $value = $row['media_quality'] ?? null;
        return is_string($value) && in_array($value, self::MEDIA_QUALITIES, true) ? $value : self::MEDIA_QUALITY_DEFAULT;
    }

    /** A boolean from a JSON body (true, 1, "1", "true", "on", "yes"); anything else = false */
    public static function flag(mixed $value): bool
    {
        return $value === true || $value === 1 || (is_string($value) && in_array(strtolower($value), ['1', 'true', 'on', 'yes'], true));
    }

    /** A valid, lower-cased email address or null */
    public static function normalizeEmail(mixed $value): ?string
    {
        $email = is_string($value) ? strtolower(trim($value)) : '';
        if ($email === '' || strlen($email) > 254 || !filter_var($email, FILTER_VALIDATE_EMAIL)) return null;
        return $email;
    }

    /**
     * The user as returned by /me/ and /auth/verify/
     * @param array<string, mixed> $row
     * @return array<string, mixed>
     */
    public function toJson(array $row): array
    {
        return [
            'id' => $row['id'],
            'email' => $row['email'],
            'created_at' => Time::iso($row['created_at']),
            'roles' => $this->roles->of((string) $row['id']),
            'marketing_opt_in' => (bool) (int) ($row['marketing_opt_in'] ?? 0),
            'hide_sensitive' => self::hidesSensitive($row),
            'media_quality' => self::mediaQuality($row),
        ];
    }

    /**
     * "Hide sensitive content" of a users row (on when the column is missing – not migrated yet – or not 0)
     * @param array<string, mixed> $row
     */
    public static function hidesSensitive(array $row): bool
    {
        return !array_key_exists('hide_sensitive', $row) || $row['hide_sensitive'] === null || (int) $row['hide_sensitive'] !== 0;
    }

    /**
     * For the admin views – also the signup code and the opt-in time
     * @param array<string, mixed> $row
     * @return array<string, mixed>
     */
    public function adminJson(array $row): array
    {
        return [
            'id' => $row['id'],
            'email' => $row['email'],
            'created_at' => Time::iso($row['created_at']),
            'roles' => $this->roles->of((string) $row['id']),
            'signup_code' => $row['signup_code'] ?? null,
            'marketing_opt_in' => (bool) (int) ($row['marketing_opt_in'] ?? 0),
            'marketing_opt_in_at' => Time::iso($row['marketing_opt_in_at'] ?? null),
            'hide_sensitive' => self::hidesSensitive($row),
            'media_quality' => self::mediaQuality($row),
        ];
    }
}
