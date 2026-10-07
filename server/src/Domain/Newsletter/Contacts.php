<?php
declare(strict_types=1);

namespace Scutoons\Domain\Newsletter;

use Scutoons\Domain\Accounts\Users;
use Scutoons\Domain\Admin\ListQuery;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Mail\MailTemplates;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Time;

/**
 * Newsletter addresses that are no account (table mail_contacts, agents/NEWSLETTER.md "Contacts"): the old QR form's
 * (source `qr-claim`, copied by the migration) and the event sign-up lists, typed in by an admin (import). Each has a
 * source (what the person signed up on) and a language – none yet = the admin sets it before a newsletter can start.
 * A contact who later gets an account is mailed as the member (one mail, the account's language).
 */
final class Contacts
{
    public const SORTABLE = ['created_at', 'email', 'source', 'locale', 'name'];
    public const FILTERS = ['source', 'locale'];
    public const MAX_IMPORT_LINES = 5000;
    public const SOURCE_PATTERN = '/^[\p{L}\p{N} .,:_\/()&+-]{1,100}$/u';

    public function __construct(private readonly Database $db, private readonly Clock $clock)
    {
    }

    /**
     * GET /api/admin/newsletter/contacts – filters: source, locale (de | en | none); q = part of the address or name
     * @return array{items: list<mixed>, pagination: array{total: int, page: int, pageSize: int}}
     */
    public function list(ListQuery $query): array
    {
        $conditions = [];
        $params = [];
        if ($query->q !== '') {
            $conditions[] = "(c.email LIKE ? ESCAPE '!' OR c.name LIKE ? ESCAPE '!')";
            $like = Database::likeContains(strtolower($query->q));
            array_push($params, $like, $like);
        }
        if (($source = $query->filter('source')) !== null) {
            $conditions[] = 'c.source = ?';
            $params[] = $source;
        }
        if (($locale = $query->filter('locale')) !== null) {
            if ($locale === 'none') {
                $conditions[] = 'c.locale IS NULL';
            } else {
                $conditions[] = 'c.locale = ?';
                $params[] = $locale;
            }
        }
        $where = $conditions ? 'WHERE ' . implode(' AND ', $conditions) : '';
        $total = (int) $this->db->value("SELECT COUNT(*) FROM mail_contacts c $where", $params);
        $rows = $this->db->all("SELECT c.*, u.id AS user_id, s.reason AS suppressed FROM mail_contacts c
            LEFT JOIN users u ON u.email = c.email LEFT JOIN mail_suppressions s ON s.email = c.email $where
            {$query->orderBy('c.' . $query->sortField, 'c.id')} LIMIT {$query->pageSize} OFFSET {$query->offset()}", $params);
        return $query->result(array_map(self::row(...), $rows), $total);
    }

    /**
     * Every source with its number of contacts and how many have no language yet
     * @return list<array{source: string, count: int, missing_locale: int}>
     */
    public function sources(): array
    {
        return array_map(fn(array $r) => [
            'source' => (string) $r['source'],
            'count' => (int) $r['n'],
            'missing_locale' => (int) $r['missing'],
        ], $this->db->all('SELECT source, COUNT(*) AS n, SUM(locale IS NULL) AS missing FROM mail_contacts GROUP BY source ORDER BY MIN(created_at), source'));
    }

    /**
     * POST /api/admin/newsletter/contacts/import – one address per line, optionally with a name and a language,
     * separated by comma, semicolon or tab, in any order ("ana@example.com, Ana, de"). Every line gets a status:
     *   new         will be added (added unless $dryRun)
     *   exists      already a contact (left as it is)
     *   member      has an account – mailed as the member
     *   suppressed  unsubscribed / bounced / blocked – not added (remove the suppression first)
     *   duplicate   the same address earlier in this list
     *   invalid     no valid address on the line
     * $locale = the language of lines that name none (null = not set yet). $consentedAt = the day the people signed up.
     * @return array{rows: list<array{line: int, email: ?string, name: ?string, locale: ?string, status: string}>, counts: array<string, int>, added: int}
     */
    public function import(mixed $text, mixed $source, mixed $consentedAt, mixed $locale, bool $dryRun, string $by): array
    {
        if (!is_string($text) || trim($text) === '') throw new ApiError(400, 'invalid_import', 'text: one email address per line.');
        $source = is_string($source) ? trim($source) : '';
        if (!preg_match(self::SOURCE_PATTERN, $source)) {
            throw new ApiError(400, 'invalid_source', 'source: where these people signed up, e.g. "event:Leipzig 2026-09" (1–100 characters).');
        }
        $consentedAt = $consentedAt === null || $consentedAt === '' ? null : $consentedAt;
        if ($consentedAt !== null && (!is_string($consentedAt) || !preg_match('/^\d{4}-\d{2}-\d{2}$/', $consentedAt))) {
            throw new ApiError(400, 'invalid_date', 'consented_at must be YYYY-MM-DD.');
        }
        $locale = self::locale($locale, true);
        $lines = preg_split('/\r\n|\r|\n/', $text) ?: [];
        if (count($lines) > self::MAX_IMPORT_LINES) {
            throw new ApiError(400, 'too_many_lines', 'At most ' . self::MAX_IMPORT_LINES . ' lines per import.', ['max' => self::MAX_IMPORT_LINES]);
        }

        $parsed = [];
        foreach ($lines as $index => $line) {
            if (trim($line) === '') continue;
            $email = null;
            $name = null;
            $lineLocale = null;
            foreach (preg_split('/[,;\t]/', $line) ?: [] as $part) {
                $part = trim($part, " \"'");
                if ($part === '') continue;
                if (str_contains($part, '@') && $email === null) {
                    $email = Users::normalizeEmail($part) ?? '';
                } elseif (in_array(strtolower($part), MailTemplates::LOCALES, true) && $lineLocale === null) {
                    $lineLocale = strtolower($part);
                } elseif ($name === null) {
                    $name = mb_substr($part, 0, 100);
                }
            }
            $parsed[] = ['line' => $index + 1, 'email' => $email !== '' ? $email : null, 'name' => $name, 'locale' => $lineLocale ?? $locale];
        }

        $emails = array_values(array_unique(array_filter(array_column($parsed, 'email'))));
        $contacts = $this->existing('SELECT email FROM mail_contacts', $emails);
        $members = $this->existing('SELECT email FROM users', $emails);
        $suppressed = $this->existing('SELECT email FROM mail_suppressions', $emails);

        $rows = [];
        $seen = [];
        $counts = array_fill_keys(['new', 'exists', 'member', 'suppressed', 'duplicate', 'invalid'], 0);
        foreach ($parsed as $row) {
            $email = $row['email'];
            $status = match (true) {
                $email === null => 'invalid',
                isset($seen[$email]) => 'duplicate',
                isset($members[$email]) => 'member',
                isset($suppressed[$email]) => 'suppressed',
                isset($contacts[$email]) => 'exists',
                default => 'new',
            };
            if ($email !== null) $seen[$email] = true;
            $counts[$status]++;
            $rows[] = $row + ['status' => $status];
        }

        $added = 0;
        if (!$dryRun) {
            $now = $this->clock->utc();
            $this->db->transaction(function () use ($rows, $source, $consentedAt, $now, $by, &$added): void {
                foreach ($rows as $row) {
                    if ($row['status'] !== 'new') continue;
                    $added += $this->db->run('INSERT IGNORE INTO mail_contacts (email, name, locale, source, consented_at, created_at, created_by) VALUES (?, ?, ?, ?, ?, ?, ?)',
                        [$row['email'], $row['name'], $row['locale'], $source, $consentedAt, $now, mb_substr($by, 0, 255)])->rowCount();
                }
            });
        }
        return ['rows' => $rows, 'counts' => $counts, 'added' => $added];
    }

    /**
     * PUT /api/admin/newsletter/contacts – { ids: [..], locale?: de|en|null, source?: string } for several at once
     * Returns how many rows changed.
     * @param array<mixed> $body
     */
    public function updateMany(array $body): int
    {
        $ids = self::ids($body['ids'] ?? null);
        $set = [];
        $params = [];
        if (array_key_exists('locale', $body)) {
            $set[] = 'locale = ?';
            $params[] = self::locale($body['locale'], true);
        }
        if (array_key_exists('source', $body)) {
            $source = is_string($body['source']) ? trim($body['source']) : '';
            if (!preg_match(self::SOURCE_PATTERN, $source)) throw new ApiError(400, 'invalid_source', 'source: 1–100 characters.');
            $set[] = 'source = ?';
            $params[] = $source;
        }
        if ($set === []) throw new ApiError(400, 'invalid_body', 'Send ids and locale and/or source.');
        $in = implode(', ', array_fill(0, count($ids), '?'));
        return $this->db->run('UPDATE mail_contacts SET ' . implode(', ', $set) . " WHERE id IN ($in)", [...$params, ...$ids])->rowCount();
    }

    /**
     * PUT /api/admin/newsletter/contacts/{id} – { name?, locale? }
     * @param array<mixed> $body
     * @return array<string, mixed> the contact
     */
    public function update(int $id, array $body): array
    {
        $this->get($id);
        if (array_key_exists('locale', $body)) $this->db->run('UPDATE mail_contacts SET locale = ? WHERE id = ?', [self::locale($body['locale'], true), $id]);
        if (array_key_exists('name', $body)) {
            $name = is_string($body['name']) && trim($body['name']) !== '' ? mb_substr(trim($body['name']), 0, 100) : null;
            $this->db->run('UPDATE mail_contacts SET name = ? WHERE id = ?', [$name, $id]);
        }
        return $this->get($id);
    }

    /** @return array<string, mixed> */
    public function get(int $id): array
    {
        $row = $this->db->one('SELECT c.*, u.id AS user_id, s.reason AS suppressed FROM mail_contacts c LEFT JOIN users u ON u.email = c.email
            LEFT JOIN mail_suppressions s ON s.email = c.email WHERE c.id = ?', [$id]);
        if ($row === null) throw new ApiError(404, 'not_found', 'No such contact.');
        return self::row($row);
    }

    /** POST /api/admin/newsletter/contacts/delete – { ids: [..] }; returns how many went */
    public function delete(mixed $ids): int
    {
        $ids = self::ids($ids);
        $in = implode(', ', array_fill(0, count($ids), '?'));
        return $this->db->run("DELETE FROM mail_contacts WHERE id IN ($in)", $ids)->rowCount();
    }

    /** de | en, or null when $nullable and the value is null / '' */
    public static function locale(mixed $value, bool $nullable): ?string
    {
        if ($nullable && ($value === null || $value === '')) return null;
        if (!is_string($value) || !in_array($value, MailTemplates::LOCALES, true)) {
            throw new ApiError(400, 'invalid_locale', 'locale must be one of ' . implode(', ', MailTemplates::LOCALES) . ($nullable ? ' (or null)' : '') . '.', ['locales' => MailTemplates::LOCALES]);
        }
        return $value;
    }

    /** @return list<int> */
    private static function ids(mixed $ids): array
    {
        if (!is_array($ids) || $ids === [] || count($ids) > 5000) throw new ApiError(400, 'invalid_ids', 'ids: a list of contact ids (1–5000).');
        $clean = [];
        foreach ($ids as $id) {
            if (!is_int($id) || $id < 1) throw new ApiError(400, 'invalid_ids', 'ids: a list of contact ids.');
            $clean[] = $id;
        }
        return array_values(array_unique($clean));
    }

    /**
     * Which of $emails are in the table of $select (a "SELECT email FROM …")
     * @param list<string> $emails
     * @return array<string, true>
     */
    private function existing(string $select, array $emails): array
    {
        $found = [];
        foreach (array_chunk($emails, 500) as $chunk) {
            $in = implode(', ', array_fill(0, count($chunk), '?'));
            foreach ($this->db->all("$select WHERE email IN ($in)", $chunk) as $row) $found[strtolower((string) $row['email'])] = true;
        }
        return $found;
    }

    /**
     * @param array<string, mixed> $r
     * @return array<string, mixed>
     */
    private static function row(array $r): array
    {
        return [
            'id' => (int) $r['id'],
            'email' => (string) $r['email'],
            'name' => $r['name'] !== null ? (string) $r['name'] : null,
            'locale' => $r['locale'] !== null ? (string) $r['locale'] : null,
            'source' => (string) $r['source'],
            'consented_at' => $r['consented_at'] !== null ? substr((string) $r['consented_at'], 0, 10) : null,
            'created_at' => Time::iso($r['created_at']),
            'created_by' => $r['created_by'] !== null ? (string) $r['created_by'] : null,
            'member' => $r['user_id'] !== null,
            'suppressed' => $r['suppressed'] !== null ? (string) $r['suppressed'] : null,
        ];
    }
}
