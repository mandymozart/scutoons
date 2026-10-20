<?php
declare(strict_types=1);

namespace Scutoons\Domain\Newsletter;

use Scutoons\Domain\Admin\ListQuery;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Mail\MailTemplates;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Time;

/**
 * Newsletters (table mail_campaigns, agents/NEWSLETTER.md): written in the admin (title, audiences, subject + body in
 * de and en), started, sent in batches by NewsletterSender.
 *   draft    everything can change; delete
 *   sending  the recipients are fixed (snapshot at the start, mail_campaign_recipients); the batches send
 *   paused   no batch sends; subject / body can still be fixed (the rest of the mails get the new text)
 *   done     every recipient is sent, failed or skipped
 * Start refuses: no recipients, a recipient without a language (a contact – set it on the Contacts page), no subject /
 * body for a language a recipient has, a `{gift}` paragraph without a gift, a gift whose claim window is over.
 * The gift (#66): `gift_campaign_id` = a giveaway campaign (Campaigns\Giveaways) or null – set in the draft.
 */
final class Campaigns
{
    public const STATUSES = ['draft', 'sending', 'paused', 'done'];
    public const RECIPIENT_SORTABLE = ['email', 'status', 'sent_at', 'locale'];
    public const RECIPIENT_FILTERS = ['status', 'locale', 'source'];
    private const MAX_BODY = 100000;

    public function __construct(private readonly Database $db, private readonly Clock $clock, private readonly Audiences $audiences)
    {
    }

    /** @return list<array<string, mixed>> every newsletter, newest first, with its progress */
    public function list(): array
    {
        $progress = $this->progress(null);
        return array_map(fn(array $r) => $this->row($r, $progress[(int) $r['id']] ?? null),
            $this->db->all('SELECT * FROM mail_campaigns ORDER BY created_at DESC, id DESC'));
    }

    /** @return array<string, mixed> */
    public function get(int $id): array
    {
        $row = $this->db->one('SELECT * FROM mail_campaigns WHERE id = ?', [$id]) ?? throw new ApiError(404, 'not_found', 'No such newsletter.');
        return $this->row($row, $this->progress($id)[$id] ?? null);
    }

    /**
     * @param array<mixed> $body { title, audiences?, subject_de?, subject_en?, body_de?, body_en? }
     * @return array<string, mixed>
     */
    public function create(array $body, string $by): array
    {
        $fields = $this->fields($body, true, 'draft');
        $now = $this->clock->utc();
        $this->db->run('INSERT INTO mail_campaigns (title, audiences, subject_de, subject_en, body_de, body_en, status, created_at, created_by, updated_at, updated_by)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)', [
            $fields['title'], $fields['audiences'] ?? '[]', $fields['subject_de'] ?? '', $fields['subject_en'] ?? '', $fields['body_de'] ?? '', $fields['body_en'] ?? '',
            'draft', $now, mb_substr($by, 0, 255), $now, mb_substr($by, 0, 255),
        ]);
        $id = $this->db->lastInsertId();
        if (array_key_exists('gift_campaign_id', $fields)) return $this->update($id, ['gift_campaign_id' => $body['gift_campaign_id']], $by);
        return $this->get($id);
    }

    /**
     * Draft: every field. Paused: subject + body only. Sending / done: nothing (409).
     * @param array<mixed> $body
     * @return array<string, mixed>
     */
    public function update(int $id, array $body, string $by): array
    {
        $current = $this->get($id);
        $fields = $this->fields($body, false, (string) $current['status']);
        if ($fields === []) return $current;
        $set = implode(', ', array_map(fn(string $column) => "$column = ?", array_keys($fields)));
        $this->db->run("UPDATE mail_campaigns SET $set, updated_at = ?, updated_by = ? WHERE id = ?",
            [...array_values($fields), $this->clock->utc(), mb_substr($by, 0, 255), $id]);
        return $this->get($id);
    }

    /** Drafts and finished newsletters can go (a finished one's recipients go with it; mail_log keeps its rows) */
    public function delete(int $id): void
    {
        $status = $this->get($id)['status'];
        if (!in_array($status, ['draft', 'done'], true)) {
            throw new ApiError(409, 'invalid_status', 'Pause is not enough: only a draft or a finished newsletter can be deleted.', ['status' => $status]);
        }
        $this->db->run('DELETE FROM mail_campaigns WHERE id = ?', [$id]);
    }

    /**
     * draft → sending: fixes the recipients (snapshot) and lets the batches send
     * @return array<string, mixed>
     */
    public function start(int $id, string $by): array
    {
        return $this->db->transaction(function () use ($id, $by): array {
            $row = $this->db->one('SELECT * FROM mail_campaigns WHERE id = ? FOR UPDATE', [$id]) ?? throw new ApiError(404, 'not_found', 'No such newsletter.');
            if ($row['status'] !== 'draft') throw new ApiError(409, 'invalid_status', 'Only a draft can be started.', ['status' => $row['status']]);
            $audiences = self::decodeAudiences((string) $row['audiences']);
            if ($audiences === []) throw new ApiError(409, 'no_audience', 'Choose who gets the newsletter.');
            $recipients = $this->audiences->recipients($audiences);
            if ($recipients === []) throw new ApiError(409, 'no_recipients', 'Nobody is in these audiences.');
            $missing = array_values(array_filter($recipients, fn(array $r) => $r['locale'] === null));
            if ($missing !== []) {
                throw new ApiError(409, 'missing_locale', count($missing) . ' contacts have no language yet – set it on the Contacts page.',
                    ['count' => count($missing), 'emails' => array_slice(array_column($missing, 'email'), 0, 20)]);
            }
            foreach (array_unique(array_column($recipients, 'locale')) as $locale) {
                if (trim((string) $row["subject_$locale"]) === '' || trim((string) $row["body_$locale"]) === '') {
                    throw new ApiError(409, 'missing_text', "Subject and text in $locale are missing – some recipients read $locale.", ['locale' => $locale]);
                }
            }
            foreach (MailTemplates::LOCALES as $locale) {
                if (($row['gift_campaign_id'] ?? null) === null && preg_match('/^[ \t]*\{gift\}[ \t]*$/m', (string) $row["body_$locale"])) {
                    throw new ApiError(409, 'gift_missing', "The $locale text has a {gift} line, but the newsletter has no gift.", ['locale' => $locale]);
                }
            }
            if (($row['gift_campaign_id'] ?? null) !== null) {
                $gift = $this->db->one('SELECT status, ends_at FROM campaigns WHERE id = ?', [$row['gift_campaign_id']]);
                if ($gift === null || $gift['status'] === 'stopped' || (string) $gift['ends_at'] <= $this->clock->utc()) {
                    throw new ApiError(409, 'gift_ended', 'The gift\'s campaign is over or stopped – choose another one.');
                }
            }
            foreach (array_chunk($recipients, 200) as $chunk) {
                $values = implode(', ', array_fill(0, count($chunk), '(?, ?, ?, ?, ?)'));
                $params = [];
                foreach ($chunk as $r) array_push($params, $id, $r['email'], $r['locale'], $r['name'] !== null ? mb_substr($r['name'], 0, 100) : null, $r['source']);
                $this->db->run("INSERT IGNORE INTO mail_campaign_recipients (campaign_id, email, locale, name, source) VALUES $values", $params);
            }
            $now = $this->clock->utc();
            $this->db->run("UPDATE mail_campaigns SET status = 'sending', started_at = ?, last_error = NULL, updated_at = ?, updated_by = ? WHERE id = ?",
                [$now, $now, mb_substr($by, 0, 255), $id]);
            return $this->get($id);
        });
    }

    /** @return array<string, mixed> */
    public function pause(int $id, string $by): array
    {
        return $this->setStatus($id, 'sending', 'paused', $by);
    }

    /** @return array<string, mixed> */
    public function resume(int $id, string $by): array
    {
        return $this->setStatus($id, 'paused', 'sending', $by);
    }

    /**
     * GET /api/admin/newsletter/campaigns/{id}/recipients – filters status, locale, source; q = part of the address
     * @return array{items: list<mixed>, pagination: array{total: int, page: int, pageSize: int}}
     */
    public function recipients(int $id, ListQuery $query): array
    {
        $this->get($id);
        $conditions = ['campaign_id = ?'];
        $params = [$id];
        if ($query->q !== '') {
            $conditions[] = "email LIKE ? ESCAPE '!'";
            $params[] = Database::likeContains(strtolower($query->q));
        }
        foreach (self::RECIPIENT_FILTERS as $name) {
            if (($value = $query->filter($name)) === null) continue;
            $conditions[] = "$name = ?";
            $params[] = $value;
        }
        $where = 'WHERE ' . implode(' AND ', $conditions);
        $total = (int) $this->db->value("SELECT COUNT(*) FROM mail_campaign_recipients $where", $params);
        $rows = $this->db->all("SELECT * FROM mail_campaign_recipients $where {$query->orderBy($query->sortField, 'id')}
            LIMIT {$query->pageSize} OFFSET {$query->offset()}", $params);
        return $query->result(array_map(fn(array $r) => [
            'id' => (int) $r['id'],
            'email' => (string) $r['email'],
            'locale' => (string) $r['locale'],
            'name' => $r['name'] !== null ? (string) $r['name'] : null,
            'source' => (string) $r['source'],
            'status' => (string) $r['status'],
            'attempts' => (int) $r['attempts'],
            'next_attempt_at' => Time::iso($r['next_attempt_at']),
            'sent_at' => Time::iso($r['sent_at']),
            'error' => $r['error'] !== null ? (string) $r['error'] : null,
        ], $rows), $total);
    }

    /** @return list<string> */
    public static function decodeAudiences(string $json): array
    {
        $list = json_decode($json, true);
        return is_array($list) ? array_values(array_filter($list, 'is_string')) : [];
    }

    /** @return array<string, mixed> */
    private function setStatus(int $id, string $from, string $to, string $by): array
    {
        $changed = $this->db->run('UPDATE mail_campaigns SET status = ?, updated_at = ?, updated_by = ? WHERE id = ? AND status = ?',
            [$to, $this->clock->utc(), mb_substr($by, 0, 255), $id, $from])->rowCount();
        $campaign = $this->get($id);
        if ($changed === 0 && $campaign['status'] !== $to) {
            throw new ApiError(409, 'invalid_status', "Only a newsletter that is $from can do that.", ['status' => $campaign['status']]);
        }
        return $campaign;
    }

    /**
     * The changeable columns in $body, checked for the newsletter's status
     * @param array<mixed> $body
     * @return array<string, string|null>
     */
    private function fields(array $body, bool $requireTitle, string $status): array
    {
        $fields = [];
        if ($requireTitle || array_key_exists('title', $body)) {
            $title = is_string($body['title'] ?? null) ? trim($body['title']) : '';
            if ($title === '' || mb_strlen($title) > 200) throw new ApiError(400, 'invalid_title', 'title: 1–200 characters (only the admin sees it).');
            $fields['title'] = $title;
        }
        if (array_key_exists('audiences', $body)) $fields['audiences'] = (string) json_encode(Audiences::check($body['audiences']), JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
        if (array_key_exists('gift_campaign_id', $body)) $fields['gift_campaign_id'] = $this->giftCampaign($body['gift_campaign_id']);
        foreach (MailTemplates::LOCALES as $locale) {
            if (array_key_exists("subject_$locale", $body)) {
                $subject = $body["subject_$locale"];
                if (!is_string($subject) || mb_strlen($subject) > 200 || preg_match('/[\r\n]/', $subject)) {
                    throw new ApiError(400, 'invalid_subject', "subject_$locale: one line, at most 200 characters.");
                }
                $fields["subject_$locale"] = trim($subject);
            }
            if (array_key_exists("body_$locale", $body)) {
                $text = $body["body_$locale"];
                if (!is_string($text) || strlen($text) > self::MAX_BODY) throw new ApiError(400, 'invalid_body', "body_$locale: text, at most 100 000 characters.");
                $fields["body_$locale"] = $text;
            }
        }
        $allowed = match ($status) {
            'draft' => null,
            'paused' => ['subject_de', 'subject_en', 'body_de', 'body_en'],
            default => [],
        };
        if ($allowed !== null && array_diff(array_keys($fields), $allowed) !== []) {
            throw new ApiError(409, 'invalid_status', $status === 'paused'
                ? 'A paused newsletter: only subject and text can still change.'
                : 'A newsletter that is sending or done can\'t change – pause it first.', ['status' => $status]);
        }
        return $fields;
    }

    /**
     * The newsletter's gift: null or the id of a giveaway campaign that is not stopped. 400 invalid_gift
     */
    private function giftCampaign(mixed $id): ?string
    {
        if ($id === null || $id === '') return null;
        $ok = is_string($id) && $this->db->value("SELECT 1 FROM campaigns WHERE id = ? AND kind = 'giveaway' AND status <> 'stopped'", [$id]) !== null;
        if (!$ok) throw new ApiError(400, 'invalid_gift', 'gift_campaign_id: a "Verschenken" campaign that is not stopped.');
        return $id;
    }

    /**
     * Recipients per status, per newsletter (null = all)
     * @return array<int, array{queued: int, sent: int, failed: int, skipped: int, total: int}>
     */
    private function progress(?int $id): array
    {
        $rows = $this->db->all('SELECT campaign_id, status, COUNT(*) AS n FROM mail_campaign_recipients'
            . ($id !== null ? ' WHERE campaign_id = ?' : '') . ' GROUP BY campaign_id, status', $id !== null ? [$id] : []);
        $progress = [];
        foreach ($rows as $r) {
            $campaign = (int) $r['campaign_id'];
            $progress[$campaign] ??= ['queued' => 0, 'sent' => 0, 'failed' => 0, 'skipped' => 0, 'total' => 0];
            $n = (int) $r['n'];
            match ((string) $r['status']) {
                'queued' => $progress[$campaign]['queued'] = $n,
                'sent' => $progress[$campaign]['sent'] = $n,
                'failed' => $progress[$campaign]['failed'] = $n,
                'skipped' => $progress[$campaign]['skipped'] = $n,
                default => null,
            };
            $progress[$campaign]['total'] += $n;
        }
        return $progress;
    }

    /**
     * @param array<string, mixed> $r
     * @param array{queued: int, sent: int, failed: int, skipped: int, total: int}|null $progress
     * @return array<string, mixed>
     */
    private function row(array $r, ?array $progress): array
    {
        return [
            'id' => (int) $r['id'],
            'title' => (string) $r['title'],
            'audiences' => self::decodeAudiences((string) $r['audiences']),
            'subject_de' => (string) $r['subject_de'],
            'subject_en' => (string) $r['subject_en'],
            'body_de' => (string) $r['body_de'],
            'body_en' => (string) $r['body_en'],
            'gift_campaign_id' => isset($r['gift_campaign_id']) ? (string) $r['gift_campaign_id'] : null,
            'status' => (string) $r['status'],
            'last_error' => $r['last_error'] !== null ? (string) $r['last_error'] : null,
            'created_at' => Time::iso($r['created_at']),
            'created_by' => $r['created_by'] !== null ? (string) $r['created_by'] : null,
            'updated_at' => Time::iso($r['updated_at']),
            'updated_by' => $r['updated_by'] !== null ? (string) $r['updated_by'] : null,
            'started_at' => Time::iso($r['started_at']),
            'finished_at' => Time::iso($r['finished_at']),
            'progress' => $progress ?? ['queued' => 0, 'sent' => 0, 'failed' => 0, 'skipped' => 0, 'total' => 0],
        ];
    }
}
