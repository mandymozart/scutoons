<?php
declare(strict_types=1);

namespace Scutoons\Domain\Mail;

use PDOException;
use Scutoons\Domain\Accounts\Users;
use Scutoons\Domain\ApiError;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Time;

/**
 * Who gets the admin notification emails (audience `admin`, Mailer::sendToAdmins – e.g. the planned
 * ownership_claim_admin). Stored as the runtime setting `mail_admin_recipients` (table `settings`, like the rate limits
 * and the homepage editions: JSON list, updated_at, updated_by = `admin:<email>`); without a row – or before the
 * `settings` migration – the default is MAIL_ADMIN_TO (comma separated, config/settings.php; empty = nobody).
 *   GET  /api/admin/emails/recipients   { items, default, stored, updated_at, updated_by, max }
 *   PUT  /api/admin/emails/recipients   { items: [email, …] | null } (null = back to the default)
 */
final class MailRecipients
{
    public const SETTING = 'mail_admin_recipients';
    public const MAX_ITEMS = 10;

    public function __construct(private readonly Database $db, private readonly Settings $config, private readonly Clock $clock)
    {
    }

    /** @return list<string> the effective addresses */
    public function admins(): array
    {
        return $this->stored()['items'] ?? $this->default();
    }

    /** @return list<string> MAIL_ADMIN_TO, valid addresses only */
    public function default(): array
    {
        return array_values(array_unique(array_filter(array_map(Users::normalizeEmail(...), $this->config->list('MAIL_ADMIN_TO')))));
    }

    /** @return array{items: list<string>, default: list<string>, stored: bool, updated_at: ?string, updated_by: ?string, max: int} */
    public function state(): array
    {
        $stored = $this->stored();
        return [
            'items' => $stored['items'] ?? $this->default(),
            'default' => $this->default(),
            'stored' => $stored !== null,
            'updated_at' => $stored !== null ? Time::iso($stored['updated_at']) : null,
            'updated_by' => $stored['updated_by'] ?? null,
            'max' => self::MAX_ITEMS,
        ];
    }

    /**
     * Replaces the list (valid addresses, lower-cased, no duplicates, at most MAX_ITEMS; an empty list = nobody), or –
     * null – goes back to the default (row removed). Returns whether anything changed.
     */
    public function update(mixed $items, string $by): bool
    {
        $stored = $this->stored();
        if ($items === null) {
            if ($stored === null) return false;
            $this->db->run('DELETE FROM settings WHERE `key` = ?', [self::SETTING]);
            return true;
        }
        if (!is_array($items) || !array_is_list($items)) {
            throw new ApiError(400, 'invalid_recipients', 'items must be a list of email addresses (or null for the default).');
        }
        $clean = [];
        foreach ($items as $index => $item) {
            $email = Users::normalizeEmail($item);
            if ($email === null) throw new ApiError(400, 'invalid_recipients', 'Not a valid email address: ' . (is_scalar($item) ? (string) $item : '?'), ['index' => $index]);
            if (!in_array($email, $clean, true)) $clean[] = $email;
        }
        if (count($clean) > self::MAX_ITEMS) {
            throw new ApiError(400, 'invalid_recipients', 'At most ' . self::MAX_ITEMS . ' addresses.', ['max' => self::MAX_ITEMS]);
        }
        if ($stored !== null && $stored['items'] === $clean) return false;
        $params = [(string) json_encode($clean, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE), $this->clock->utc(), substr($by, 0, 255), self::SETTING];
        if ($stored !== null) $this->db->run('UPDATE settings SET `value` = ?, updated_at = ?, updated_by = ? WHERE `key` = ?', $params);
        else $this->db->run('INSERT INTO settings (`value`, updated_at, updated_by, `key`) VALUES (?, ?, ?, ?)', $params);
        return true;
    }

    /** @return array{items: list<string>, updated_at: string, updated_by: ?string}|null */
    private function stored(): ?array
    {
        try {
            $row = $this->db->one('SELECT `value`, updated_at, updated_by FROM settings WHERE `key` = ?', [self::SETTING]);
        } catch (PDOException) {
            return null; // no settings table yet (migration pending): the default
        }
        if ($row === null) return null;
        $items = json_decode((string) $row['value'], true);
        if (!is_array($items) || !array_is_list($items)) return null;
        return [
            'items' => array_values(array_filter($items, 'is_string')),
            'updated_at' => (string) $row['updated_at'],
            'updated_by' => $row['updated_by'] !== null ? (string) $row['updated_by'] : null,
        ];
    }
}
