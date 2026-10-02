<?php
declare(strict_types=1);

namespace Scutoons\Domain\Mail;

use PDOException;
use Scutoons\Domain\Admin\ListQuery;
use Scutoons\Domain\ApiError;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Time;

/**
 * The sent-mail log (table `mail_log`, migration 2026-10-02-mail-log): one row per mail Mailer tried to send –
 * template, recipient, locale, status (sent | failed), transport, error, created_at. Never the subject, the text or
 * the variables (a login mail carries a one-time code and a sign-in link).
 *
 * Privacy: the recipient address is stored (the admin needs it to answer "I got no mail"); it is shown only to
 * admins (like the users endpoints) and kept MAIL_LOG_DAYS days (default 180, pruned on every write).
 * Before the migration ran the table is missing: logging is skipped, sending still works.
 */
final class MailLog
{
    public const STATUSES = ['sent', 'failed'];
    public const SORTABLE = ['created_at', 'template', 'status', 'recipient'];
    public const FILTERS = ['template', 'status', 'recipient', 'from', 'to'];

    public function __construct(private readonly Database $db, private readonly Clock $clock, private readonly Settings $settings)
    {
    }

    /** Records one attempt; returns the row id (0 = not logged: no table yet / database error) */
    public function record(string $template, string $to, string $locale, string $status, string $transport, ?string $error): int
    {
        try {
            $days = $this->settings->int('MAIL_LOG_DAYS');
            if ($days > 0) $this->db->run('DELETE FROM mail_log WHERE created_at < ?', [$this->clock->utc(-86400 * $days)]);
            $this->db->run('INSERT INTO mail_log (template, recipient, locale, status, transport, error, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)', [
                substr($template, 0, 64), mb_substr($to, 0, 254), substr($locale, 0, 8), $status, substr($transport, 0, 16),
                $error !== null ? mb_substr($error, 0, 500) : null, $this->clock->utc(),
            ]);
            return $this->db->lastInsertId();
        } catch (PDOException $exception) {
            error_log('[qr-api] mail log not written: ' . $exception->getMessage());
            return 0;
        }
    }

    /**
     * The admin list (GET /api/admin/emails/log): newest first by default; q = part of the recipient
     * @return array{items: list<mixed>, pagination: array{total: int, page: int, pageSize: int}}
     */
    public function list(ListQuery $query): array
    {
        $conditions = [];
        $params = [];
        if ($query->q !== '') {
            $conditions[] = "recipient LIKE ? ESCAPE '!'";
            $params[] = Database::likeContains(strtolower($query->q));
        }
        if (($template = $query->filter('template')) !== null) {
            if (!MailTemplates::exists($template)) throw new ApiError(400, 'invalid_template', "Unknown email template $template.", ['templates' => MailTemplates::ids()]);
            $conditions[] = 'template = ?';
            $params[] = $template;
        }
        if (($status = $query->filter('status')) !== null) {
            if (!in_array($status, self::STATUSES, true)) throw new ApiError(400, 'invalid_status', "Unknown status $status.", ['statuses' => self::STATUSES]);
            $conditions[] = 'status = ?';
            $params[] = $status;
        }
        if (($recipient = $query->filter('recipient')) !== null) {
            $conditions[] = 'recipient = ?';
            $params[] = strtolower(trim($recipient));
        }
        foreach (['from' => '>=', 'to' => '<='] as $name => $operator) {
            $value = $query->filter($name);
            if ($value === null) continue;
            if (!preg_match('/^\d{4}-\d{2}-\d{2}$/', $value)) throw new ApiError(400, 'invalid_date', "$name must be YYYY-MM-DD (UTC).");
            $conditions[] = "created_at $operator ?";
            $params[] = $value . ($name === 'from' ? ' 00:00:00' : ' 23:59:59');
        }
        $where = $conditions ? 'WHERE ' . implode(' AND ', $conditions) : '';
        try {
            $total = (int) $this->db->value("SELECT COUNT(*) FROM mail_log $where", $params);
            $rows = $this->db->all("SELECT id, template, recipient, locale, status, transport, error, created_at FROM mail_log $where
                {$query->orderBy($query->sortField, 'id')} LIMIT {$query->pageSize} OFFSET {$query->offset()}", $params);
        } catch (PDOException) {
            return $query->result([], 0); // no mail_log table yet (migration pending)
        }
        return $query->result(array_map(fn(array $r) => [
            'id' => (int) $r['id'],
            'template' => (string) $r['template'],
            'recipient' => (string) $r['recipient'],
            'locale' => (string) $r['locale'],
            'status' => (string) $r['status'],
            'transport' => (string) $r['transport'],
            'error' => $r['error'] !== null ? (string) $r['error'] : null,
            'created_at' => Time::iso($r['created_at']),
        ], $rows), $total);
    }
}
