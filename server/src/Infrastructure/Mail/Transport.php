<?php
declare(strict_types=1);

namespace Scutoons\Infrastructure\Mail;

use RuntimeException;
use Scutoons\Infrastructure\Config\Settings;

/**
 * Delivers one rendered email (text + HTML). Platform mails go through the email module (Domain\Mail\Mailer: template →
 * render → deliver here → mail_log); the one other caller is the Embassy's person-written mail (deliverText, logged
 * in mail_log as template `embassy` by Domain\Embassy\EmbassyMail). MAIL_TRANSPORT:
 * - `log`: appends the mail to MAIL_LOG_PATH – local development (the smoke tests and scripts/dev/test-user.mjs read
 *   the one-time codes there; keep the entry format: "=== <ISO time> ===", headers, To, Subject, blank line, text)
 * - `mail`: PHP mail() – quick tests on a host that has it set up
 * - `smtp`: the SMTP account in SMTP_* – production (Smtp, no library needed)
 * Batches (the newsletter, Domain\Newsletter\NewsletterSender): beginBatch() … deliverWithHeaders() … endBatch() keeps
 * one SMTP connection open for all of them (a failed one is reopened by the next mail).
 */
class Transport
{
    /** The connection of a batch (null with $batching = none open yet) */
    private ?Smtp $batch = null;
    private bool $batching = false;

    public function __construct(private readonly Settings $settings)
    {
    }

    /** From now on SMTP mails share one connection until endBatch() (log / mail: no difference) */
    public function beginBatch(): void
    {
        $this->batching = true;
    }

    public function endBatch(): void
    {
        $this->batch?->close();
        $this->batch = null;
        $this->batching = false;
    }

    /** How many mails the batch's open connection has delivered (0 = none open) */
    public function batchDelivered(): int
    {
        return $this->batch !== null && $this->batch->isOpen() ? $this->batch->delivered() : 0;
    }

    /** Closes the batch's connection; the next mail opens a new one (the sender does it every NEWSLETTER_PER_CONNECTION mails) */
    public function reconnect(): void
    {
        $this->batch?->close();
    }

    /** The configured transport: log | mail | smtp */
    public function name(): string
    {
        return $this->settings->get('MAIL_TRANSPORT');
    }

    /** @throws RuntimeException when the mail could not be handed over */
    public function deliver(string $to, string $subject, string $text, string $html): void
    {
        $message = $this->message($subject, $text, $html);
        $this->hand($this->settings->get('MAIL_FROM'), $to, $subject, $message['headers'], $message['body'], $text);
    }

    /**
     * deliver() with more headers (the newsletter's List-Unsubscribe / List-Unsubscribe-Post)
     * @param list<string> $headers whole lines, e.g. "List-Unsubscribe-Post: List-Unsubscribe=One-Click"
     * @throws RuntimeException when the mail could not be handed over (SmtpFailure with smtp)
     */
    public function deliverWithHeaders(string $to, string $subject, string $text, string $html, array $headers): void
    {
        $message = $this->message($subject, $text, $html, $headers);
        $this->hand($this->settings->get('MAIL_FROM'), $to, $subject, $message['headers'], $message['body'], $text);
    }

    /**
     * Delivers one plain-text mail a person wrote (the Embassy, Domain\Embassy\EmbassyMail): UTF-8, quoted-printable,
     * from its own address and SMTP account, threaded with In-Reply-To / References. Same transports as deliver().
     * @param array{from: string, from_name?: string, reply_to?: string, message_id: string, in_reply_to?: ?string, references?: list<string>, user?: ?string, pass?: ?string} $envelope
     *        message_id / in_reply_to / references without the angle brackets; user / pass null = SMTP_USER / SMTP_PASS
     * @return string the whole message as handed over (headers, To, Subject, body) – e.g. for a copy in the Sent folder
     * @throws RuntimeException when the mail could not be handed over
     */
    public function deliverText(string $to, string $subject, string $text, array $envelope): string
    {
        $from = $envelope['from'];
        $name = $envelope['from_name'] ?? '';
        $references = array_values(array_filter($envelope['references'] ?? [], fn(string $id) => $id !== ''));
        $replyTo = $envelope['reply_to'] ?? '';
        $inReplyTo = $envelope['in_reply_to'] ?? '';
        $headers = implode("\r\n", [
            'From: ' . ($name !== '' ? self::encodeHeader($name) . " <$from>" : "<$from>"),
            ...(filter_var($replyTo, FILTER_VALIDATE_EMAIL) ? ["Reply-To: <$replyTo>"] : []),
            'Date: ' . date(DATE_RFC2822),
            'Message-ID: <' . $envelope['message_id'] . '>',
            ...($inReplyTo !== '' ? ["In-Reply-To: <$inReplyTo>"] : []),
            ...($references ? ['References: ' . implode(' ', array_map(fn(string $id) => "<$id>", $references))] : []),
            'MIME-Version: 1.0',
            'Content-Type: text/plain; charset=UTF-8',
            'Content-Transfer-Encoding: quoted-printable',
        ]);
        // CRLF line breaks stay hard breaks (a bare LF would be encoded as =0A)
        $body = quoted_printable_encode(str_replace("\n", "\r\n", str_replace(["\r\n", "\r"], "\n", $text)));
        $this->hand($from, $to, $subject, $headers, $body, $text, $envelope['user'] ?? null, $envelope['pass'] ?? null);
        return $headers . "\r\nTo: $to\r\nSubject: " . self::encodeHeader($subject) . "\r\n\r\n" . $body;
    }

    /**
     * Hands a prepared message to MAIL_TRANSPORT. $headers without To / Subject (mail() adds those); $text = the plain
     * text for the log file; $user / $pass = the SMTP account (null = SMTP_USER / SMTP_PASS)
     */
    private function hand(string $from, string $to, string $subject, string $headers, string $body, string $text, ?string $user = null, ?string $pass = null): void
    {
        $transport = $this->name();
        switch ($transport) {
            case 'log':
                $path = $this->settings->path('MAIL_LOG_PATH') ?: sys_get_temp_dir() . '/scutoons-mail.log';
                if (!is_dir(dirname($path))) @mkdir(dirname($path), 0770, true);
                $entry = "=== " . gmdate('c') . " ===\r\n" . $headers . "\r\nTo: $to\r\nSubject: $subject\r\n\r\n$text\r\n\r\n";
                if (@file_put_contents($path, $entry, FILE_APPEND | LOCK_EX) === false) {
                    throw new RuntimeException("Mail log not writable: $path");
                }
                return;
            case 'mail':
                $ok = mail($to, self::encodeHeader($subject), $body, $headers, '-f' . $from);
                if (!$ok) throw new RuntimeException('mail() failed');
                return;
            case 'smtp':
                $data = $headers . "\r\nTo: $to\r\nSubject: " . self::encodeHeader($subject) . "\r\n\r\n" . $body;
                if (!$this->batching) {
                    (new Smtp($this->settings))->send($from, $to, $data, $user, $pass);
                    return;
                }
                $this->batch ??= new Smtp($this->settings);
                $this->batch->open($user, $pass);
                $this->batch->deliver($from, $to, $data);
                return;
            default:
                throw new RuntimeException("Unknown MAIL_TRANSPORT: $transport");
        }
    }

    /**
     * Headers (without To / Subject – mail() adds those) and the multipart body
     * @param list<string> $extra more header lines (lines with a line break are dropped)
     * @return array{headers: string, body: string}
     */
    private function message(string $subject, string $text, string $html, array $extra = []): array
    {
        $boundary = 'scutoons-' . bin2hex(random_bytes(12));
        $from = $this->settings->get('MAIL_FROM');
        $at = strrchr($from, '@');
        $domain = substr($at !== false ? $at : '@localhost', 1);
        $replyTo = $this->settings->get('MAIL_REPLY_TO');
        $headers = implode("\r\n", [
            'From: ' . self::encodeHeader($this->settings->get('MAIL_FROM_NAME')) . " <$from>",
            // replies ("reply to this email" in the texts) reach a read mailbox, not no-reply@
            ...(filter_var($replyTo, FILTER_VALIDATE_EMAIL) ? ["Reply-To: <$replyTo>"] : []),
            'Date: ' . date(DATE_RFC2822),
            'Message-ID: <' . bin2hex(random_bytes(16)) . "@$domain>",
            ...array_values(array_filter($extra, fn(string $line) => $line !== '' && !preg_match('/[\r\n]/', $line))),
            'MIME-Version: 1.0',
            "Content-Type: multipart/alternative; boundary=\"$boundary\"",
        ]);
        $part = fn(string $type, string $content) =>
            "--$boundary\r\nContent-Type: $type; charset=UTF-8\r\nContent-Transfer-Encoding: base64\r\n\r\n"
            . chunk_split(base64_encode($content)) . "\r\n";
        $body = $part('text/plain', $text) . $part('text/html', $html) . "--$boundary--\r\n";
        return ['headers' => $headers, 'body' => $body];
    }

    /** RFC 2047 encoded-word (base64) for a header value with non-ASCII characters */
    public static function encodeHeader(string $value): string
    {
        return preg_match('/[^\x20-\x7e]/', $value) ? '=?UTF-8?B?' . base64_encode($value) . '?=' : $value;
    }
}
