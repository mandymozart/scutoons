<?php
declare(strict_types=1);

/**
 * Minimal SMTP client (RFC 5321): STARTTLS or implicit TLS, AUTH LOGIN, one recipient. Enough for the
 * login mails – no library on the host needed. Settings: SMTP_HOST, SMTP_PORT, SMTP_SECURE, SMTP_USER, SMTP_PASS.
 */
final class Smtp
{
    /** @var resource|null */
    private $socket = null;

    public function send(string $from, string $to, string $data): void
    {
        $host = Config::get('SMTP_HOST');
        $port = Config::int('SMTP_PORT');
        $secure = Config::get('SMTP_SECURE');
        $timeout = Config::int('SMTP_TIMEOUT');
        if ($host === '') throw new RuntimeException('SMTP_HOST is not set');

        $address = ($secure === 'ssl' ? 'ssl://' : 'tcp://') . "$host:$port";
        $socket = @stream_socket_client($address, $errno, $errstr, $timeout);
        if (!$socket) throw new RuntimeException("SMTP connect to $address failed: $errstr ($errno)");
        $this->socket = $socket;
        stream_set_timeout($socket, $timeout);

        try {
            $this->expect(220);
            $ehlo = 'EHLO ' . (gethostname() ?: 'localhost');
            $this->command($ehlo, 250);
            if ($secure === 'tls') {
                $this->command('STARTTLS', 220);
                if (!stream_socket_enable_crypto($socket, true, STREAM_CRYPTO_METHOD_TLSv1_2_CLIENT | STREAM_CRYPTO_METHOD_TLSv1_3_CLIENT)) {
                    throw new RuntimeException('SMTP STARTTLS failed');
                }
                $this->command($ehlo, 250);
            }
            $user = Config::get('SMTP_USER');
            if ($user !== '') {
                $this->command('AUTH LOGIN', 334);
                $this->command(base64_encode($user), 334);
                $this->command(base64_encode(Config::get('SMTP_PASS')), 235, 'AUTH (password hidden)');
            }
            $this->command("MAIL FROM:<$from>", 250);
            $this->command("RCPT TO:<$to>", [250, 251]);
            $this->command('DATA', 354);
            // Dot-stuffing: a line starting with "." gets a second one
            $body = preg_replace('/^\./m', '..', str_replace(["\r\n", "\n"], ["\n", "\r\n"], $data));
            $this->write(rtrim((string) $body, "\r\n") . "\r\n.\r\n");
            $this->expect(250);
            $this->command('QUIT', 221);
        } finally {
            fclose($socket);
            $this->socket = null;
        }
    }

    /** @param int|int[] $expected */
    private function command(string $line, int|array $expected, ?string $logAs = null): void
    {
        $this->write("$line\r\n");
        $this->expect($expected, $logAs ?? $line);
    }

    private function write(string $data): void
    {
        if (fwrite($this->socket, $data) === false) throw new RuntimeException('SMTP write failed');
    }

    /** Reads a (multi-line) reply and checks its code @param int|int[] $expected */
    private function expect(int|array $expected, string $after = 'connect'): string
    {
        $reply = '';
        while (($line = fgets($this->socket, 1024)) !== false) {
            $reply .= $line;
            if (strlen($line) < 4 || $line[3] === ' ') break;
        }
        $code = (int) substr($reply, 0, 3);
        if (!in_array($code, (array) $expected, true)) {
            throw new RuntimeException("SMTP: unexpected reply to $after: " . trim($reply));
        }
        return $reply;
    }
}
