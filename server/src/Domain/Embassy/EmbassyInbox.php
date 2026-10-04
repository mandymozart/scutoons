<?php
declare(strict_types=1);

namespace Scutoons\Domain\Embassy;

use Scutoons\Domain\ApiError;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Infrastructure\Mail\Mailbox;
use Scutoons\Infrastructure\Mail\MimeMessage;
use Scutoons\Support\Time;
use Throwable;

/**
 * The inbox sync (agents/EMBASSY.md "Automation": "IMAP sync: replies matched to leads, sequence stopped"): reads the
 * new mails of the shared mailbox (EMBASSY_IMAP_*; POST /api/embassy/inbox/sync or `bin/console embassy:inbox` from a
 * cron job) and logs each reply as a `mail_in` activity on the organisation it answers – the cadence then moves the
 * lead to "in conversation" (the follow-up sequence stops).
 *
 * Matching: (a) In-Reply-To / References name one of our mail_out mails → its organisation, person and track; (b) else
 * the sender is a person's or an organisation's address → that one, the track of the organisation's latest mail_out.
 * Unknown senders, our own address, automatic replies (out of office, bounces) are skipped and NOTHING about them is
 * stored – only counted (privacy). A mail already logged (same Message-ID) is skipped. A reply whose first line is just
 * "stop" / "stopp" / "unsubscribe" / "abmelden" sets do-not-contact on the person (or the organisation) – forever.
 *
 * Where it stopped: embassy_mail_state (per folder: UIDVALIDITY + the last UID read; a new UIDVALIDITY starts over).
 * The first run reads only the last FIRST_DAYS days. Needs migration 2026-10-04-ask-embassy-mail (503 otherwise).
 */
final class EmbassyInbox
{
    /** Largest part of a mail read (the text is at the start; attachments are not needed) */
    public const MAX_BYTES = 524288;
    public const NOTE_MAX = 4000;
    /** A sync from the app runs at most every … seconds (ambassadors may force one) */
    public const THROTTLE_SECONDS = 120;
    public const FIRST_DAYS = 30;
    /** Mails per run – the rest on the next run (keeps a request short) */
    public const MAX_PER_RUN = 200;
    private const STOP_WORDS = ['stop', 'stopp', 'unsubscribe', 'abmelden'];

    public function __construct(
        private readonly Embassy $embassy,
        private readonly Database $db,
        private readonly Clock $clock,
        private readonly Settings $settings,
        private readonly Mailbox $mailbox,
    ) {
    }

    public function configured(): bool
    {
        return $this->settings->get('EMBASSY_IMAP_HOST') !== '';
    }

    /** @return array{configured: bool, synced_at: ?string, error: ?string} */
    public function status(): array
    {
        $state = $this->configured() ? $this->state() : null;
        return ['configured' => $this->configured(), 'synced_at' => Time::iso($state['synced_at'] ?? null),
            'error' => isset($state['error']) ? (string) $state['error'] : null];
    }

    /**
     * A sync started from the app (Embassy access): skipped while the last one is younger than THROTTLE_SECONDS,
     * unless an ambassador forces it
     * @param array<string, mixed> $user
     * @return array<string, mixed> like run() (+ throttled)
     */
    public function sync(array $user, bool $force): array
    {
        $access = $this->embassy->access($user);
        if (!$this->configured()) return ['configured' => false];
        $this->requireAvailable();
        $last = $this->state()['synced_at'] ?? null;
        $age = is_string($last) ? $this->clock->time() - (int) strtotime($last . ' UTC') : PHP_INT_MAX;
        if ($age < self::THROTTLE_SECONDS && !($force && $access === Embassy::AMBASSADOR)) {
            return $this->status() + ['matched' => 0, 'skipped' => 0, 'new_activities' => 0, 'throttled' => true];
        }
        return $this->run() + ['throttled' => false];
    }

    /**
     * Reads the new mails of EMBASSY_IMAP_FOLDER and logs the replies
     * @return array{configured: bool, synced_at?: ?string, matched?: int, skipped?: int, new_activities?: int, error?: ?string}
     */
    public function run(): array
    {
        if (!$this->configured()) return ['configured' => false];
        $this->requireAvailable();
        $folder = $this->folder();
        $state = $this->state();
        $validity = isset($state['uid_validity']) ? (int) $state['uid_validity'] : null;
        $lastUid = (int) ($state['last_uid'] ?? 0);
        $counts = ['matched' => 0, 'skipped' => 0, 'new_activities' => 0];
        $error = null;
        try {
            $this->mailbox->open();
            $current = $this->mailbox->select($folder);
            if ($current !== $validity) $lastUid = 0; // a new UIDVALIDITY: the old UIDs mean nothing any more
            $validity = $current;
            $since = gmdate('Y-m-d', $this->clock->time() - self::FIRST_DAYS * 86400);
            foreach (array_slice($this->mailbox->search($lastUid, $since), 0, self::MAX_PER_RUN) as $uid) {
                $raw = $this->mailbox->fetch($uid, self::MAX_BYTES);
                try {
                    $outcome = $this->import($raw);
                } catch (Throwable $e) {
                    error_log("[qr-api] embassy inbox: mail $uid not logged: " . $e->getMessage());
                    $outcome = 'skipped';
                }
                $counts[$outcome]++;
                if ($outcome === 'matched') $counts['new_activities']++;
                $lastUid = max($lastUid, $uid);
            }
        } catch (Throwable $e) {
            $error = mb_substr($e->getMessage(), 0, 500);
            error_log('[qr-api] embassy inbox sync failed: ' . $e->getMessage());
        } finally {
            $this->mailbox->close();
        }
        $now = $this->clock->utc();
        $this->saveState($folder, $validity, $lastUid, $now, $error);
        return ['configured' => true, 'synced_at' => Time::iso($now)] + $counts + ['error' => $error];
    }

    /**
     * One raw mail → a mail_in activity, or skipped (unknown sender, automatic, ours, already logged)
     * @return 'matched'|'skipped'
     */
    public function import(string $raw): string
    {
        $mail = MimeMessage::parse($raw);
        $from = $mail->address('From');
        if ($from === '' || $from === strtolower($this->settings->get('EMBASSY_MAIL_FROM')) || self::automatic($mail, $from)) return 'skipped';
        $messageId = $mail->ids('Message-ID')[0] ?? 'sha1-' . sha1($raw) . '@inbox';
        if ($this->db->value('SELECT 1 FROM embassy_activities WHERE message_id = ?', [$messageId]) !== null) return 'skipped';
        $match = $this->match($mail, $from);
        if ($match === null) return 'skipped';

        $reply = self::reply($mail->text());
        $note = mb_substr($reply, 0, self::NOTE_MAX);
        $data = ['kind' => 'mail_in', 'track' => $match['track'], 'person_id' => $match['person_id'], 'note' => $note];
        if (self::isStop($reply)) {
            $this->embassy->optOut($match['org_id'], $match['person_id']);
            $who = $match['person_id'] !== null ? 'this person' : 'the organisation';
            $data['note'] = trim($note . "\n\n[Opt-out: they replied “stop” – $who is marked do-not-contact.]");
            $data['next_at'] = null; // no follow-up
            if ($match['person_id'] === null) $data['stage'] = 'no';
        }
        $this->embassy->logMail($match['org_id'], $data, null, [
            'subject' => $mail->header('Subject'), 'mail_from' => $from, 'message_id' => $messageId,
            'in_reply_to' => $mail->ids('In-Reply-To')[0] ?? null, 'at' => $this->date($mail->header('Date')), 'by_email' => 'inbox',
        ]);
        return 'matched';
    }

    /**
     * The reply without the quoted mail under it: lines starting with ">" and everything from "On … wrote:",
     * "Am … schrieb …:", "-----Original Message-----" or an Outlook header block ("Von:" / "From:" + "Gesendet:" …) on
     */
    public static function reply(string $text): string
    {
        $lines = explode("\n", str_replace(["\r\n", "\r"], "\n", $text));
        $out = [];
        foreach ($lines as $i => $line) {
            $t = trim($line);
            $next = trim($lines[$i + 1] ?? '');
            if (str_starts_with($t, '>')) continue;
            if (preg_match('/^On\b.{0,300}\bwrote:$/iu', $t) || (preg_match('/^On\b/i', $t) && mb_strlen($t) < 300 && preg_match('/\bwrote:$/i', $next))) break;
            if (preg_match('/^Am\b.{0,300}\bschrieb\b.{0,300}:$/iu', $t) || (preg_match('/^Am\b/', $t) && mb_strlen($t) < 300 && preg_match('/\bschrieb\b.*:$/iu', $next))) break;
            if (preg_match('/^-{2,}\s*(Original Message|Ursprüngliche Nachricht|Forwarded message|Weitergeleitete Nachricht)\s*-{2,}/iu', $t)) break;
            if (preg_match('/^_{10,}$/', $t)) break; // Outlook's separator line
            if (preg_match('/^(Von|From):\s/i', $t)) {
                $block = implode("\n", array_slice($lines, $i + 1, 4));
                if (preg_match('/^\s*(Gesendet|Sent|An|To|Datum|Date|Betreff|Subject):\s/imu', $block)) break;
            }
            $out[] = rtrim($line);
        }
        return trim((string) preg_replace("/\n{3,}/", "\n\n", implode("\n", $out)));
    }

    /** The first non-empty line is just stop / stopp / unsubscribe / abmelden (any case, punctuation ok) */
    public static function isStop(string $reply): bool
    {
        foreach (explode("\n", $reply) as $line) {
            if (trim($line) === '') continue;
            return in_array((string) preg_replace('/[^\p{L}]+/u', '', mb_strtolower($line)), self::STOP_WORDS, true);
        }
        return false;
    }

    // ── Internals ─────────────────────────────────────────────────────────────────────────────────────────

    /**
     * The organisation (+ person, track) a mail answers – null = unknown sender
     * @return array{org_id: string, person_id: ?string, track: ?string}|null
     */
    private function match(MimeMessage $mail, string $from): ?array
    {
        $person = $this->db->one('SELECT id, org_id FROM embassy_people WHERE LOWER(email) = ? ORDER BY created_at, id LIMIT 1', [$from]);
        $ids = array_values(array_unique([...$mail->ids('In-Reply-To'), ...$mail->ids('References')]));
        if ($ids) {
            $thread = $this->db->one("SELECT a.org_id, a.person_id, l.track FROM embassy_activities a LEFT JOIN embassy_leads l ON l.id = a.lead_id
                WHERE a.kind = 'mail_out' AND a.message_id IN (" . implode(', ', array_fill(0, count($ids), '?')) . ') ORDER BY a.at DESC, a.id DESC LIMIT 1', $ids);
            if ($thread !== null) {
                $orgId = (string) $thread['org_id'];
                // a colleague may answer: the sender when they are a person of that organisation, else whom we wrote to
                $sender = $this->db->value('SELECT id FROM embassy_people WHERE org_id = ? AND LOWER(email) = ? ORDER BY created_at, id LIMIT 1', [$orgId, $from]);
                $personId = $sender !== null ? (string) $sender : ($thread['person_id'] !== null ? (string) $thread['person_id'] : null);
                return ['org_id' => $orgId, 'person_id' => $personId, 'track' => $thread['track'] !== null ? (string) $thread['track'] : null];
            }
        }
        if ($person !== null) {
            [$orgId, $personId] = [(string) $person['org_id'], (string) $person['id']];
        } else {
            $org = $this->db->value('SELECT id FROM embassy_orgs WHERE LOWER(email) = ? ORDER BY created_at, id LIMIT 1', [$from]);
            if ($org === null) return null;
            [$orgId, $personId] = [(string) $org, null];
        }
        $track = $this->db->value("SELECT l.track FROM embassy_activities a JOIN embassy_leads l ON l.id = a.lead_id
            WHERE a.org_id = ? AND a.kind = 'mail_out' ORDER BY a.at DESC, a.id DESC LIMIT 1", [$orgId]);
        return ['org_id' => $orgId, 'person_id' => $personId, 'track' => $track !== null ? (string) $track : null];
    }

    /** Out-of-office answers, bounces, mailing lists: not a reply of a person */
    private static function automatic(MimeMessage $mail, string $from): bool
    {
        $auto = strtolower($mail->header('Auto-Submitted'));
        if ($auto !== '' && $auto !== 'no') return true;
        if ($mail->has('X-Autoreply') || $mail->has('X-Autorespond')) return true;
        if (in_array(strtolower($mail->header('Precedence')), ['bulk', 'junk', 'list', 'auto_reply'], true)) return true;
        return (bool) preg_match('/^(mailer-daemon|postmaster)@/i', $from);
    }

    /** The mail's Date as UTC DATETIME (missing, broken or in the future: now) */
    private function date(string $header): string
    {
        $time = $header !== '' ? strtotime($header) : false;
        if ($time === false || $time > $this->clock->time() + 86400) $time = $this->clock->time();
        return gmdate('Y-m-d H:i:s', $time);
    }

    private function folder(): string
    {
        return mb_substr($this->settings->get('EMBASSY_IMAP_FOLDER') ?: 'INBOX', 0, 64);
    }

    /** 503 before migration 2026-10-04-ask-embassy-mail */
    private function requireAvailable(): void
    {
        try {
            $this->db->value('SELECT COUNT(*) FROM embassy_mail_state WHERE 1 = 0');
            $ok = $this->embassy->hasMailColumns();
        } catch (Throwable) {
            $ok = false;
        }
        if (!$ok) throw new ApiError(503, 'not_available', 'The inbox sync is not set up yet (migration 2026-10-04-ask-embassy-mail).');
    }

    /** @return array<string, mixed>|null this folder's row (null: none yet, or no table) */
    private function state(): ?array
    {
        try {
            return $this->db->one('SELECT * FROM embassy_mail_state WHERE id = ?', [$this->folder()]);
        } catch (Throwable) {
            return null;
        }
    }

    private function saveState(string $folder, ?int $validity, int $lastUid, string $now, ?string $error): void
    {
        $values = [$validity, $lastUid, $now, $error, $folder];
        if ($this->db->value('SELECT 1 FROM embassy_mail_state WHERE id = ?', [$folder]) !== null) {
            $this->db->run('UPDATE embassy_mail_state SET uid_validity = ?, last_uid = ?, synced_at = ?, error = ? WHERE id = ?', $values);
        } else {
            $this->db->run('INSERT INTO embassy_mail_state (uid_validity, last_uid, synced_at, error, id) VALUES (?, ?, ?, ?, ?)', $values);
        }
    }
}
