<?php
declare(strict_types=1);

namespace Scutoons\Domain\Embassy;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Mail\MailLog;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Mail\Imap;
use Scutoons\Infrastructure\Mail\Mailbox;
use Scutoons\Infrastructure\Mail\Transport;
use Scutoons\Support\Ids;
use Throwable;

/**
 * Sending from the Embassy app (agents/EMBASSY.md "Communication strategy" / "Automation", owner 2026-10-04): a person
 * with Embassy access writes one mail to one organisation – to its address or one of its people's – and it goes out
 * from the shared mailbox (EMBASSY_MAIL_FROM, its own SMTP account), plain text, threaded when it answers a logged mail.
 * No approval step; the AI agent only drafts – sending is always this endpoint, called when the person clicks Send.
 *
 * Rules: the recipient must belong to the organisation (400 invalid_recipient), nobody marked do-not-contact is ever
 * mailed (409 do_not_contact – the opt-out is respected forever), every mail ends with the opt-out line (FOOTERS, added
 * unless the text has it already). After the transport took it: a mail_log row (template `embassy`), the `mail_out`
 * activity through Embassy::logMail (cadence: contacted + call in 6 days) and, when EMBASSY_IMAP_SENT is set, a copy in
 * that IMAP folder (best effort). A transport failure → 502 mail_failed and nothing logged on the organisation.
 */
final class EmbassyMail
{
    public const TEMPLATE = 'embassy';
    public const SUBJECT_MAX = 300;
    public const TEXT_MAX = 20000;
    /** The opt-out line under every mail (agents/EMBASSY.md "Legal": opt-out in every mail) */
    public const FOOTERS = [
        'de' => "—\nKeine weiteren Nachrichten von uns? Einfach mit „Stopp“ antworten.",
        'en' => "—\nRather not hear from us? Just reply “stop”.",
    ];
    /** A text containing one of these already has the opt-out line */
    private const FOOTER_MARKS = ['keine weiteren nachrichten von uns', 'rather not hear from us'];
    /** Countries whose organisations get the German footer when nobody's language is known */
    private const GERMAN_COUNTRIES = ['DE', 'AT', 'CH', 'LI'];

    public function __construct(
        private readonly Embassy $embassy,
        private readonly Transport $transport,
        private readonly MailLog $log,
        private readonly Settings $settings,
        private readonly Mailbox $mailbox,
    ) {
    }

    /** The sender address (EMBASSY_MAIL_FROM) */
    public function from(): string
    {
        return $this->settings->get('EMBASSY_MAIL_FROM');
    }

    /**
     * Sends one mail and logs it. Body `{ to, person_id?, track?, subject, text, reply_to_activity_id?, language? }`
     * (`language` de | en = the footer's language; default: the person's language, else German for DE / AT / CH / LI
     * organisations, else English).
     * @param array<string, mixed> $data
     * @param array<string, mixed> $user
     * @return array{activity: array<string, mixed>, lead: array<string, mixed>|null}
     */
    public function send(string $orgId, array $data, array $user): array
    {
        $this->embassy->access($user);
        $detail = $this->embassy->get($orgId);
        $org = $detail['org'];

        $to = trim((string) ($data['to'] ?? ''));
        if ($to === '' || filter_var($to, FILTER_VALIDATE_EMAIL) === false) {
            throw new ApiError(400, 'invalid_recipient', 'Not an email address.', ['field' => 'to']);
        }
        $toKey = strtolower($to);
        $owner = null; // the person whose address it is
        foreach ($detail['people'] as $person) {
            if ($person['email'] !== '' && strtolower((string) $person['email']) === $toKey) {
                $owner = $person;
                break;
            }
        }
        if ($owner === null && ($org['email'] === '' || strtolower((string) $org['email']) !== $toKey)) {
            throw new ApiError(400, 'invalid_recipient', "Mails go to the organisation's address or one of its people's.", ['field' => 'to']);
        }
        $person = $owner;
        $personId = isset($data['person_id']) && $data['person_id'] !== '' ? (string) $data['person_id'] : null;
        if ($personId !== null) {
            $person = null;
            foreach ($detail['people'] as $candidate) {
                if ($candidate['id'] === $personId) $person = $candidate;
            }
            if ($person === null) throw new ApiError(400, 'invalid_person', 'This person is not part of the organisation.');
        }
        if ($org['do_not_contact'] || ($person['do_not_contact'] ?? false) || ($owner['do_not_contact'] ?? false)) {
            throw new ApiError(409, 'do_not_contact', 'They asked not to be contacted.');
        }

        $subject = trim((string) preg_replace('/[\r\n\t]+/', ' ', (string) ($data['subject'] ?? '')));
        if ($subject === '') throw new ApiError(400, 'subject_required', 'The mail needs a subject.', ['field' => 'subject']);
        if (mb_strlen($subject) > self::SUBJECT_MAX) throw new ApiError(400, 'subject_too_long', 'At most ' . self::SUBJECT_MAX . ' characters.', ['field' => 'subject']);
        $text = trim(str_replace(["\r\n", "\r"], "\n", (string) ($data['text'] ?? '')));
        if ($text === '') throw new ApiError(400, 'text_required', 'The mail needs a text.', ['field' => 'text']);
        if (mb_strlen($text) > self::TEXT_MAX) throw new ApiError(400, 'text_too_long', 'At most ' . self::TEXT_MAX . ' characters.', ['field' => 'text']);
        $track = isset($data['track']) && $data['track'] !== '' ? (string) $data['track'] : null;
        $this->embassy->assertMayLog($orgId, $track, $user);

        // threading: the mail answers a logged mail of this organisation
        $parent = null;
        $replyTo = isset($data['reply_to_activity_id']) && $data['reply_to_activity_id'] !== '' ? (string) $data['reply_to_activity_id'] : null;
        if ($replyTo !== null) {
            $row = $this->embassy->activityRow($replyTo);
            if ($row === null || (string) $row['org_id'] !== $orgId || !in_array($row['kind'], ['mail_in', 'mail_out'], true)) {
                throw new ApiError(400, 'invalid_reply_to', 'reply_to_activity_id is not a mail of this organisation.', ['field' => 'reply_to_activity_id']);
            }
            $parent = $row;
        }
        $parentId = ($parent['message_id'] ?? null) !== null && $parent['message_id'] !== '' ? (string) $parent['message_id'] : null;
        $references = $parentId !== null ? array_values(array_filter([(string) ($parent['in_reply_to'] ?? ''), $parentId], fn(string $id) => $id !== '')) : [];

        $language = self::language($data['language'] ?? null, $person, $org);
        $body = self::withFooter($text, $language);
        $from = $this->from();
        $at = strrchr($from, '@');
        $messageId = Ids::uuid7() . '@' . substr($at !== false ? $at : '@scutoons.com', 1);
        $account = $this->settings->get('EMBASSY_SMTP_USER');
        try {
            $raw = $this->transport->deliverText($to, $subject, $body, [
                'from' => $from, 'from_name' => $this->settings->get('EMBASSY_MAIL_FROM_NAME'), 'reply_to' => $from,
                'message_id' => $messageId, 'in_reply_to' => $parentId, 'references' => $references,
                'user' => $account !== '' ? $account : null, 'pass' => $account !== '' ? $this->settings->get('EMBASSY_SMTP_PASS') : null,
            ]);
        } catch (Throwable $error) {
            $this->log->record(self::TEMPLATE, $to, $language, 'failed', $this->transport->name(), $error->getMessage());
            error_log('[qr-api] embassy mail failed: ' . $error->getMessage());
            throw new ApiError(502, 'mail_failed', 'The mail could not be sent: ' . mb_substr($error->getMessage(), 0, 200));
        }
        $this->log->record(self::TEMPLATE, $to, $language, 'sent', $this->transport->name(), null);
        $result = $this->embassy->logMail($orgId, ['kind' => 'mail_out', 'track' => $track, 'person_id' => $person['id'] ?? null, 'note' => $body], $user,
            ['subject' => $subject, 'mail_from' => $from, 'mail_to' => $to, 'message_id' => $messageId, 'in_reply_to' => $parentId]);
        $this->copyToSent($raw);
        return $result;
    }

    /** The text with the opt-out line under it (unless it has one) */
    public static function withFooter(string $text, string $language): string
    {
        $lower = mb_strtolower($text);
        foreach (self::FOOTER_MARKS as $mark) {
            if (str_contains($lower, $mark)) return $text;
        }
        return rtrim($text) . "\n\n" . (self::FOOTERS[$language] ?? self::FOOTERS['en']);
    }

    /**
     * de | en: the asked language, else the person's, else by the organisation's country, else en
     * @param array<string, mixed>|null $person
     * @param array<string, mixed> $org
     */
    private static function language(mixed $asked, ?array $person, array $org): string
    {
        foreach ([$asked, $person['language'] ?? null] as $value) {
            $code = strtolower(substr(trim(is_string($value) ? $value : ''), 0, 2));
            if (isset(self::FOOTERS[$code])) return $code;
        }
        return in_array($org['country'] ?? '', self::GERMAN_COUNTRIES, true) ? 'de' : 'en';
    }

    /** A copy in the shared mailbox's Sent folder (EMBASSY_IMAP_SENT) – best effort, never fails the send */
    private function copyToSent(string $raw): void
    {
        $folder = $this->settings->get('EMBASSY_IMAP_SENT');
        if ($folder === '' || Imap::host($this->settings) === '') return;
        try {
            $this->mailbox->open();
            $this->mailbox->append($folder, $raw);
        } catch (Throwable $error) {
            error_log('[qr-api] embassy mail: no copy in ' . $folder . ': ' . $error->getMessage());
        } finally {
            $this->mailbox->close();
        }
    }
}
