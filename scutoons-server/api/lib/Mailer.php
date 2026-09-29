<?php
declare(strict_types=1);

/**
 * Sends one email (text + HTML). Ported from osct server/api/src/Mailer.php. MAIL_TRANSPORT:
 * - `log`: appends the mail to MAIL_LOG_PATH (default: the system temp folder) – local development
 * - `mail`: PHP mail() – quick tests on a host that has it set up
 * - `smtp`: the SMTP account in SMTP_* – production (Smtp.php, no library needed)
 */
final class Mailer
{
    public static function send(string $to, string $subject, string $text, string $html): void
    {
        $transport = Config::get('MAIL_TRANSPORT');
        $message = self::message($to, $subject, $text, $html);
        switch ($transport) {
            case 'log':
                $path = Config::get('MAIL_LOG_PATH') ?: sys_get_temp_dir() . '/scutoons-mail.log';
                if (!is_dir(dirname($path))) @mkdir(dirname($path), 0770, true);
                $entry = "=== " . gmdate('c') . " ===\r\n" . $message['headers'] . "\r\nTo: $to\r\nSubject: $subject\r\n\r\n$text\r\n\r\n";
                if (file_put_contents($path, $entry, FILE_APPEND | LOCK_EX) === false) {
                    throw new RuntimeException("Mail log not writable: $path");
                }
                return;
            case 'mail':
                $ok = mail($to, self::encodeHeader($subject), $message['body'], $message['headers'], '-f' . Config::get('MAIL_FROM'));
                if (!$ok) throw new RuntimeException('mail() failed');
                return;
            case 'smtp':
                $data = $message['headers'] . "\r\nTo: $to\r\nSubject: " . self::encodeHeader($subject) . "\r\n\r\n" . $message['body'];
                (new Smtp())->send(Config::get('MAIL_FROM'), $to, $data);
                return;
            default:
                throw new RuntimeException("Unknown MAIL_TRANSPORT: $transport");
        }
    }

    /**
     * Headers (without To / Subject – mail() adds those) and the multipart body
     * @return array{headers: string, body: string}
     */
    private static function message(string $to, string $subject, string $text, string $html): array
    {
        $boundary = 'scutoons-' . bin2hex(random_bytes(12));
        $from = Config::get('MAIL_FROM');
        $domain = substr(strrchr($from, '@') ?: '@localhost', 1);
        $headers = implode("\r\n", [
            'From: ' . self::encodeHeader(Config::get('MAIL_FROM_NAME')) . " <$from>",
            'Date: ' . date(DATE_RFC2822),
            'Message-ID: <' . bin2hex(random_bytes(16)) . "@$domain>",
            'MIME-Version: 1.0',
            "Content-Type: multipart/alternative; boundary=\"$boundary\"",
        ]);
        $part = fn(string $type, string $content) =>
            "--$boundary\r\nContent-Type: $type; charset=UTF-8\r\nContent-Transfer-Encoding: base64\r\n\r\n"
            . chunk_split(base64_encode($content)) . "\r\n";
        $body = $part('text/plain', $text) . $part('text/html', $html) . "--$boundary--\r\n";
        return ['headers' => $headers, 'body' => $body];
    }

    private static function encodeHeader(string $value): string
    {
        return preg_match('/[^\x20-\x7e]/', $value) ? '=?UTF-8?B?' . base64_encode($value) . '?=' : $value;
    }
}
