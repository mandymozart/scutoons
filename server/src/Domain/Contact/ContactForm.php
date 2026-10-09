<?php
declare(strict_types=1);

namespace Scutoons\Domain\Contact;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Mail\MailLog;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Mail\Transport;
use Throwable;

/**
 * The public contact form (owner 2026-10-09; scutoons.com/kontakt – also the imprint's second contact route next to
 * the email address, ECG § 5). A message becomes one plain-text mail to CONTACT_TO (default MAIL_REPLY_TO =
 * hello@scutoons.com) with the sender as Reply-To – answered from the mailbox, nothing stored in the database (only
 * the mail log line, like every mail).
 *
 * Spam, without a third-party CAPTCHA: a honeypot field (`website`) and a minimum time on the page (`elapsed` ms)
 * – both drop the message silently (the bot sees success) – plus a per-IP limit per hour (config/routes.php). No
 * confirmation mail to the sender: the form must not be usable to mail strangers.
 */
final class ContactForm
{
    public const TOPICS = ['general', 'code', 'account', 'privacy', 'press'];
    private const TOPIC_LABELS = [
        'general' => 'Allgemein',
        'code' => 'QR-Code / Code',
        'account' => 'Konto & Mitgliedschaft',
        'privacy' => 'Datenschutz-Anfrage',
        'press' => 'Presse & Booking',
    ];
    /** faster than this (ms from opening the form to sending) is a bot */
    private const MIN_ELAPSED_MS = 3000;
    private const MAX_MESSAGE = 5000;

    public function __construct(
        private readonly Transport $transport,
        private readonly MailLog $log,
        private readonly Settings $settings,
    ) {
    }

    /**
     * @param array<string, mixed> $data { topic, email, name?, code?, message, locale?, page?, website (honeypot), elapsed }
     * @param array<string, mixed>|null $user the signed-in reader, if any
     * @return bool true = sent; false = dropped as spam (the caller answers success either way)
     * @throws ApiError 400 invalid_topic | invalid_email | message_required | message_too_long; 503 mail_failed
     */
    public function submit(array $data, ?array $user): bool
    {
        if (trim((string) ($data['website'] ?? '')) !== '') return false;
        if ((int) ($data['elapsed'] ?? 0) < self::MIN_ELAPSED_MS) return false;

        $topic = (string) ($data['topic'] ?? 'general');
        if (!in_array($topic, self::TOPICS, true)) throw new ApiError(400, 'invalid_topic', 'Unknown topic.', ['allowed' => self::TOPICS]);
        $email = mb_substr(trim((string) ($data['email'] ?? '')), 0, 255);
        if ($email === '' || filter_var($email, FILTER_VALIDATE_EMAIL) === false) throw new ApiError(400, 'invalid_email', 'Please enter a valid email address.', ['field' => 'email']);
        $message = trim(str_replace(["\r\n", "\r"], "\n", (string) ($data['message'] ?? '')));
        if (mb_strlen($message) < 5) throw new ApiError(400, 'message_required', 'Please write a message.', ['field' => 'message']);
        if (mb_strlen($message) > self::MAX_MESSAGE) throw new ApiError(400, 'message_too_long', 'At most ' . self::MAX_MESSAGE . ' characters.', ['field' => 'message']);
        $name = self::line($data['name'] ?? '', 100);
        $code = self::line($data['code'] ?? '', 64);
        $locale = ($data['locale'] ?? '') === 'en' ? 'en' : 'de';
        $page = self::line($data['page'] ?? '', 300);

        $label = self::TOPIC_LABELS[$topic];
        $subject = "[Kontakt · $label] " . mb_substr(preg_replace('/\s+/', ' ', $message) ?? '', 0, 60);
        $lines = [
            "Thema: $label",
            'Von: ' . ($name !== '' ? "$name <$email>" : $email),
            ...($code !== '' ? ["Code: $code"] : []),
            'Konto: ' . ($user !== null ? 'angemeldet (' . (string) ($user['email'] ?? '') . ', id ' . (string) ($user['id'] ?? '') . ')' : 'nicht angemeldet'),
            "Sprache: $locale",
            ...($page !== '' ? ["Seite: $page"] : []),
            '',
            $message,
            '',
            '–',
            'Über das Kontaktformular auf scutoons.com. Antworten geht direkt an den Absender.',
        ];

        $to = $this->settings->get('CONTACT_TO') ?: $this->settings->get('MAIL_REPLY_TO');
        $from = $this->settings->get('MAIL_FROM');
        $domain = substr((string) strrchr($from, '@'), 1) ?: 'localhost';
        try {
            $this->transport->deliverText($to, $subject, implode("\n", $lines), [
                'from' => $from,
                'from_name' => 'SCUTOONS Kontakt',
                'reply_to' => $email,
                'message_id' => bin2hex(random_bytes(16)) . '@' . $domain,
            ]);
        } catch (Throwable $error) {
            $this->log->record('contact', $to, $locale, 'failed', $this->transport->name(), $error->getMessage());
            error_log('[qr-api] contact mail failed: ' . $error->getMessage());
            throw new ApiError(503, 'mail_failed', 'The message could not be sent – please write to hello@scutoons.com.');
        }
        $this->log->record('contact', $to, $locale, 'sent', $this->transport->name(), null);
        return true;
    }

    /** one line of text: no line breaks, trimmed, at most $max characters */
    private static function line(mixed $value, int $max): string
    {
        return mb_substr(trim((string) preg_replace('/[\r\n\t]+/', ' ', is_scalar($value) ? (string) $value : '')), 0, $max);
    }
}
