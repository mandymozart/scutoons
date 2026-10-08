<?php
declare(strict_types=1);

namespace Scutoons\Infrastructure\Mail;

use Scutoons\Infrastructure\Config\Settings;

/**
 * Minimal SMTP client (RFC 5321): STARTTLS or implicit TLS, AUTH LOGIN, one recipient per mail. Enough for the login
 * mails – no library on the host needed. Settings: SMTP_HOST, SMTP_PORT, SMTP_SECURE, SMTP_USER, SMTP_PASS, SMTP_TIMEOUT.
 * Another account on the same server (the Embassy's shared mailbox) passes its own user / password.
 *
 * One mail: send() (connect, log in, deliver, QUIT). Many mails (the newsletter batches): open() once, deliver() each
 * (RSET between them), close() – one connection and one login for the whole batch instead of one per mail.
 * Failures throw SmtpFailure (the step + the reply code).
 */
final class Smtp
{
    /** @var resource|null */
    private $socket = null;
    private int $delivered = 0;

    public function __construct(private readonly Settings $settings)
    {
    }

    /** $user / $pass: null = SMTP_USER / SMTP_PASS */
    public function send(string $from, string $to, string $data, ?string $user = null, ?string $pass = null): void
    {
        $this->open($user, $pass);
        try {
            $this->deliver($from, $to, $data);
            $this->command('QUIT', 221, null, 'data');
        } finally {
            $this->drop();
        }
    }

    public function isOpen(): bool
    {
        return $this->socket !== null;
    }

    /** How many mails this connection has delivered */
    public function delivered(): int
    {
        return $this->delivered;
    }

    /** Connects and logs in. $user / $pass: null = SMTP_USER / SMTP_PASS */
    public function open(?string $user = null, ?string $pass = null): void
    {
        if ($this->socket !== null) return;
        $host = $this->settings->get('SMTP_HOST');
        $port = $this->settings->int('SMTP_PORT');
        $secure = $this->settings->get('SMTP_SECURE');
        $timeout = $this->settings->int('SMTP_TIMEOUT');
        if ($host === '') throw new SmtpFailure('SMTP_HOST is not set', 'connect');

        $address = ($secure === 'ssl' ? 'ssl://' : 'tcp://') . "$host:$port";
        $socket = @stream_socket_client($address, $errno, $errstr, $timeout);
        if (!$socket) throw new SmtpFailure("SMTP connect to $address failed: $errstr ($errno)", 'connect');
        $this->socket = $socket;
        $this->delivered = 0;
        stream_set_timeout($socket, $timeout);

        try {
            $this->expect(220, 'connect', 'connect');
            $ehlo = 'EHLO ' . (gethostname() ?: 'localhost');
            $this->command($ehlo, 250, null, 'connect');
            if ($secure === 'tls') {
                $this->command('STARTTLS', 220, null, 'tls');
                if (!stream_socket_enable_crypto($socket, true, STREAM_CRYPTO_METHOD_TLSv1_2_CLIENT | STREAM_CRYPTO_METHOD_TLSv1_3_CLIENT)) {
                    throw new SmtpFailure('SMTP STARTTLS failed', 'tls');
                }
                $this->command($ehlo, 250, null, 'tls');
            }
            $user ??= $this->settings->get('SMTP_USER');
            if ($user !== '') {
                $this->command('AUTH LOGIN', 334, null, 'auth');
                $this->command(base64_encode($user), 334, null, 'auth');
                $this->command(base64_encode($pass ?? $this->settings->get('SMTP_PASS')), 235, 'AUTH (password hidden)', 'auth');
            }
        } catch (SmtpFailure $failure) {
            $this->drop();
            throw $failure;
        }
    }

    /**
     * Delivers one mail on the open connection. A refused recipient (RCPT) leaves the connection usable – the next
     * deliver() starts with RSET; any other failure closes it (the next open() connects again).
     */
    public function deliver(string $from, string $to, string $data): void
    {
        if ($this->socket === null) throw new SmtpFailure('SMTP not connected', 'connect');
        try {
            if ($this->delivered > 0) $this->command('RSET', 250, null, 'mail');
            $this->command("MAIL FROM:<$from>", 250, null, 'mail');
            $this->command("RCPT TO:<$to>", [250, 251], null, 'rcpt');
            $this->command('DATA', 354, null, 'data');
            // Dot-stuffing: a line starting with "." gets a second one
            $body = preg_replace('/^\./m', '..', str_replace(["\r\n", "\n"], ["\n", "\r\n"], $data));
            $this->write(rtrim((string) $body, "\r\n") . "\r\n.\r\n", 'data');
            $this->expect(250, 'end of data', 'data');
        } catch (SmtpFailure $failure) {
            if ($failure->stage === 'rcpt' && $this->socket !== null) {
                $this->delivered++; // RSET before the next one
            } else {
                $this->drop();
            }
            throw $failure;
        }
        $this->delivered++;
    }

    /** Says goodbye and closes the connection (no error when it is already gone) */
    public function close(): void
    {
        if ($this->socket === null) return;
        try {
            $this->command('QUIT', 221, null, 'data');
        } catch (SmtpFailure) {
            // closing anyway
        }
        $this->drop();
    }

    private function drop(): void
    {
        if ($this->socket !== null) @fclose($this->socket);
        $this->socket = null;
    }

    /** @param int|list<int> $expected */
    private function command(string $line, int|array $expected, ?string $logAs, string $stage): void
    {
        $this->write("$line\r\n", $stage);
        $this->expect($expected, $logAs ?? $line, $stage);
    }

    private function write(string $data, string $stage): void
    {
        if ($this->socket === null || fwrite($this->socket, $data) === false) throw new SmtpFailure('SMTP write failed', $stage);
    }

    /**
     * Reads a (multi-line) reply and checks its code
     * @param int|list<int> $expected
     */
    private function expect(int|array $expected, string $after, string $stage): string
    {
        if ($this->socket === null) throw new SmtpFailure('SMTP not connected', $stage);
        $reply = '';
        while (($line = fgets($this->socket, 1024)) !== false) {
            $reply .= $line;
            if (strlen($line) < 4 || $line[3] === ' ') break;
        }
        $code = (int) substr($reply, 0, 3);
        if (!in_array($code, (array) $expected, true)) {
            throw new SmtpFailure("SMTP: unexpected reply to $after: " . trim($reply), $stage, $code);
        }
        return $reply;
    }
}
