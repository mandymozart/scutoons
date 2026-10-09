<?php
declare(strict_types=1);

namespace Scutoons\Domain\Embassy;

use Scutoons\Domain\ApiError;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Ids;
use Throwable;

/**
 * Mail drafts on an organisation (agents/MCP.md "Draft → link", owner 2026-10-09): the member's own AI saves a draft
 * through the API (the same checks as a send: EmbassyMail::prepare – recipient of the organisation, do-not-contact,
 * subject, text) and hands the member the link; the member opens it on the organisation page, edits it and sends it
 * (POST /orgs/{id}/mail with `draft_id` – the draft is deleted once the mail went out). Nothing is ever sent from here.
 * Every Embassy member sees an organisation's drafts; only the author sends or discards one (an ambassador may discard
 * any). `source` names who wrote it: 'agent' (the AI, through a personal token) or 'person'.
 */
final class EmbassyDrafts
{
    public function __construct(
        private readonly Database $db,
        private readonly Clock $clock,
        private readonly Embassy $embassy,
        private readonly EmbassyMail $mail,
    ) {
    }

    /** Before migration 2026-10-09-mcp-drafts the endpoints answer 503 */
    private function requireAvailable(): void
    {
        try {
            $this->db->value('SELECT COUNT(*) FROM embassy_mail_drafts WHERE 1 = 0');
        } catch (Throwable) {
            throw new ApiError(503, 'not_available', 'Mail drafts are not set up yet (migration 2026-10-09-mcp-drafts).');
        }
    }

    /**
     * A new draft `{ to, person_id?, track?, subject, text, reply_to_activity_id?, language?, source? }` – checked like a send
     * @param array<string, mixed> $data
     * @param array<string, mixed> $user
     * @return array<string, mixed>
     */
    public function create(string $orgId, array $data, array $user): array
    {
        $this->requireAvailable();
        $mail = $this->mail->prepare($orgId, $data, $user);
        $id = Ids::uuid7();
        $now = $this->clock->utc();
        $source = ($data['source'] ?? '') === 'agent' ? 'agent' : 'person';
        $language = strtolower(substr(trim(is_string($data['language'] ?? null) ? $data['language'] : ''), 0, 2));
        $this->db->run(
            'INSERT INTO embassy_mail_drafts (id, org_id, person_id, mail_to, track, subject, text, reply_to_activity_id, language, source, created_by, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)',
            [$id, $orgId, $mail['person']['id'] ?? null, $mail['to'], $mail['track'], $mail['subject'], $mail['text'], $mail['reply_to'],
                in_array($language, ['de', 'en'], true) ? $language : null, $source, (string) $user['id'], $now, $now]
        );
        return $this->draft($this->row($id));
    }

    /**
     * The open drafts – of one organisation, or every one (the start page's "Drafts (n)")
     * @return list<array<string, mixed>>
     */
    public function list(?string $orgId = null): array
    {
        try {
            $rows = $orgId === null
                ? $this->db->all('SELECT d.*, o.name AS org_name, u.email AS author_email FROM embassy_mail_drafts d JOIN embassy_orgs o ON o.id = d.org_id LEFT JOIN users u ON u.id = d.created_by ORDER BY d.created_at DESC LIMIT 200')
                : $this->db->all('SELECT d.*, o.name AS org_name, u.email AS author_email FROM embassy_mail_drafts d JOIN embassy_orgs o ON o.id = d.org_id LEFT JOIN users u ON u.id = d.created_by WHERE d.org_id = ? ORDER BY d.created_at DESC', [$orgId]);
        } catch (Throwable) {
            return []; // before the migration: no drafts
        }
        return array_map(fn(array $r) => $this->draft($r), $rows);
    }

    /** @return array<string, mixed> */
    public function get(string $id): array
    {
        $this->requireAvailable();
        return $this->draft($this->row($id));
    }

    /**
     * The author (or an ambassador) discards a draft
     * @param array<string, mixed> $user
     */
    public function delete(string $id, array $user): void
    {
        $this->requireAvailable();
        $row = $this->row($id);
        if ((string) $row['created_by'] !== (string) $user['id'] && $this->embassy->access($user) !== Embassy::AMBASSADOR) {
            throw new ApiError(403, 'forbidden', 'Only its author (or an ambassador) discards a draft.');
        }
        $this->db->run('DELETE FROM embassy_mail_drafts WHERE id = ?', [$id]);
    }

    /**
     * Before a send with `draft_id`: the draft belongs to this organisation and to the sender (a draft goes out in its
     * author's name only)
     * @param array<string, mixed> $user
     */
    public function assertSendable(string $id, string $orgId, array $user): void
    {
        $this->requireAvailable();
        $row = $this->row($id);
        if ((string) $row['org_id'] !== $orgId) throw new ApiError(400, 'invalid_draft', 'The draft belongs to another organisation.');
        if ((string) $row['created_by'] !== (string) $user['id']) throw new ApiError(403, 'forbidden', 'Only its author sends a draft – copy the text into a mail of your own.');
    }

    /** After the mail went out */
    public function sent(string $id): void
    {
        $this->db->run('DELETE FROM embassy_mail_drafts WHERE id = ?', [$id]);
    }

    /** @return array<string, mixed> */
    private function row(string $id): array
    {
        return $this->db->one('SELECT d.*, o.name AS org_name, u.email AS author_email FROM embassy_mail_drafts d JOIN embassy_orgs o ON o.id = d.org_id LEFT JOIN users u ON u.id = d.created_by WHERE d.id = ?', [$id])
            ?? throw new ApiError(404, 'not_found', 'No such draft (sent or discarded already?).');
    }

    /**
     * @param array<string, mixed> $r
     * @return array<string, mixed>
     */
    private function draft(array $r): array
    {
        return [
            'id' => (string) $r['id'],
            'org_id' => (string) $r['org_id'],
            'org' => (string) ($r['org_name'] ?? ''),
            'person_id' => $r['person_id'] !== null ? (string) $r['person_id'] : null,
            'to' => (string) $r['mail_to'],
            'track' => $r['track'] !== null ? (string) $r['track'] : null,
            'subject' => (string) $r['subject'],
            'text' => (string) $r['text'],
            'reply_to_activity_id' => $r['reply_to_activity_id'] !== null ? (string) $r['reply_to_activity_id'] : null,
            'language' => $r['language'] !== null ? (string) $r['language'] : null,
            'source' => (string) $r['source'],
            'created_by' => (string) $r['created_by'],
            'author' => (string) ($r['author_email'] ?? ''),
            'created_at' => (string) $r['created_at'],
        ];
    }
}
