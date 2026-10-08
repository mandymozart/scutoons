<?php
declare(strict_types=1);

namespace Scutoons\Domain\Newsletter;

use Scutoons\Domain\Accounts\Users;
use Scutoons\Domain\Admin\ListQuery;
use Scutoons\Domain\ApiError;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Time;

/**
 * Addresses no newsletter goes to again (table mail_suppressions, agents/NEWSLETTER.md "Unsubscribe and bounces"):
 *   unsubscribe  the link in the mail / the mail client's one-click button (List-Unsubscribe-Post)
 *   bounce       the SMTP server refused the address for good (5xx at RCPT TO)
 *   manual       added by an admin
 * A member who unsubscribes also gets marketing_opt_in = 0; ticking "Send me news" again on the account page lifts
 * their unsubscribe (Users::setMarketingOptIn). Login and other service mails (Domain\Mail) are never suppressed.
 */
final class Suppressions
{
    public const REASONS = ['unsubscribe', 'bounce', 'manual'];
    public const SORTABLE = ['created_at', 'email', 'reason'];
    public const FILTERS = ['reason'];

    public function __construct(private readonly Database $db, private readonly Clock $clock)
    {
    }

    /** Adds (or keeps) the address; an unsubscribe also clears a member's opt-in. Returns whether it is new. */
    public function add(string $email, string $reason, ?string $detail = null): bool
    {
        $email = Users::normalizeEmail($email) ?? strtolower(trim($email));
        if ($email === '') return false;
        if (!in_array($reason, self::REASONS, true)) throw new ApiError(400, 'invalid_reason', 'reason must be one of ' . implode(', ', self::REASONS) . '.');
        $added = $this->db->run('INSERT IGNORE INTO mail_suppressions (email, reason, detail, created_at) VALUES (?, ?, ?, ?)',
            [mb_substr($email, 0, 254), $reason, $detail !== null ? mb_substr($detail, 0, 500) : null, $this->clock->utc()])->rowCount() === 1;
        if ($reason === 'unsubscribe') {
            $now = $this->clock->utc();
            $this->db->run('UPDATE users SET marketing_opt_in = 0, marketing_opt_in_at = ?, updated_at = ? WHERE email = ? AND marketing_opt_in = 1', [$now, $now, $email]);
        }
        return $added;
    }

    public function remove(string $email): bool
    {
        return $this->db->run('DELETE FROM mail_suppressions WHERE email = ?', [strtolower(trim($email))])->rowCount() > 0;
    }

    public function has(string $email): bool
    {
        return $this->db->value('SELECT 1 FROM mail_suppressions WHERE email = ?', [strtolower(trim($email))]) !== null;
    }

    /** @return array{items: list<mixed>, pagination: array{total: int, page: int, pageSize: int}} */
    public function list(ListQuery $query): array
    {
        $conditions = [];
        $params = [];
        if ($query->q !== '') {
            $conditions[] = "email LIKE ? ESCAPE '!'";
            $params[] = Database::likeContains(strtolower($query->q));
        }
        if (($reason = $query->filter('reason')) !== null) {
            $conditions[] = 'reason = ?';
            $params[] = $reason;
        }
        $where = $conditions ? 'WHERE ' . implode(' AND ', $conditions) : '';
        $total = (int) $this->db->value("SELECT COUNT(*) FROM mail_suppressions $where", $params);
        $rows = $this->db->all("SELECT email, reason, detail, created_at FROM mail_suppressions $where
            {$query->orderBy($query->sortField, 'email')} LIMIT {$query->pageSize} OFFSET {$query->offset()}", $params);
        return $query->result(array_map(fn(array $r) => [
            'email' => (string) $r['email'],
            'reason' => (string) $r['reason'],
            'detail' => $r['detail'] !== null ? (string) $r['detail'] : null,
            'created_at' => Time::iso($r['created_at']),
        ], $rows), $total);
    }
}
