<?php
declare(strict_types=1);

namespace Scutoons\Infrastructure\Mail;

use RuntimeException;
use Scutoons\Infrastructure\Config\Settings;

/**
 * Minimal IMAP4rev1 client (RFC 3501) over stream_socket_client – the host has no ext-imap. Implicit TLS (port 993;
 * port 143 = plain, local test servers only), LOGIN (quoted strings, literals for anything else), EXAMINE (= a read-only
 * SELECT), UID SEARCH, UID FETCH BODY.PEEK[] (partial, never marks a mail as read), APPEND, LOGOUT. Settings:
 * EMBASSY_IMAP_HOST, EMBASSY_IMAP_PORT, EMBASSY_IMAP_USER / _PASS (empty = EMBASSY_SMTP_USER / _PASS, then SMTP_USER /
 * _PASS), SMTP_TIMEOUT (seconds per read).
 */
final class Imap implements Mailbox
{
    /** @var resource|null */
    private $socket = null;
    private int $tag = 0;

    public function __construct(private readonly Settings $settings)
    {
    }

    public function open(): void
    {
        $host = $this->settings->get('EMBASSY_IMAP_HOST');
        if ($host === '') throw new RuntimeException('EMBASSY_IMAP_HOST is not set');
        $port = $this->settings->int('EMBASSY_IMAP_PORT') ?: 993;
        $timeout = max(5, $this->settings->int('SMTP_TIMEOUT'));
        $address = ($port === 143 ? 'tcp://' : 'ssl://') . "$host:$port";
        $context = stream_context_create(['ssl' => ['verify_peer' => true, 'verify_peer_name' => true, 'SNI_enabled' => true, 'peer_name' => $host]]);
        $socket = @stream_socket_client($address, $errno, $errstr, $timeout, STREAM_CLIENT_CONNECT, $context);
        if (!$socket) throw new RuntimeException("IMAP connect to $address failed: $errstr ($errno)");
        $this->socket = $socket;
        stream_set_timeout($socket, $timeout);
        $greeting = $this->line();
        if (!str_starts_with($greeting, '* OK') && !str_starts_with($greeting, '* PREAUTH')) {
            $this->close();
            throw new RuntimeException('IMAP: unexpected greeting: ' . trim($greeting));
        }
        [$user, $pass] = self::credentials($this->settings);
        try {
            $this->command([['LOGIN '], ...self::astring($user), [' '], ...self::astring($pass)], 'LOGIN (password hidden)');
        } catch (RuntimeException $e) {
            $this->close();
            throw $e;
        }
    }

    public function select(string $folder): int
    {
        $response = $this->command([['EXAMINE '], ...self::astring($folder)]);
        foreach ($response['lines'] as $line) {
            if (preg_match('/\[UIDVALIDITY (\d+)\]/i', $line, $m)) return (int) $m[1];
        }
        throw new RuntimeException("IMAP: no UIDVALIDITY for $folder");
    }

    public function search(int $afterUid, string $since): array
    {
        $criteria = $afterUid > 0 ? 'UID ' . ($afterUid + 1) . ':*' : 'SINCE ' . gmdate('j-M-Y', (int) strtotime($since . ' 00:00:00 UTC'));
        $uids = [];
        foreach ($this->command([["UID SEARCH $criteria"]])['lines'] as $line) {
            if (!preg_match('/^\* SEARCH\b(.*)$/i', $line, $m)) continue;
            foreach (preg_split('/\s+/', trim($m[1])) ?: [] as $uid) {
                // "n:*" always answers the newest mail, even when its UID is not above n
                if (ctype_digit($uid) && (int) $uid > $afterUid) $uids[] = (int) $uid;
            }
        }
        sort($uids);
        return array_values(array_unique($uids));
    }

    public function fetch(int $uid, int $maxBytes): string
    {
        $response = $this->command([["UID FETCH $uid (BODY.PEEK[]<0.$maxBytes>)"]]);
        return $response['literals'][0] ?? '';
    }

    public function append(string $folder, string $message): void
    {
        $this->command([['APPEND '], ...self::astring($folder), [' (\\Seen) '], ['', $message]]);
    }

    public function close(): void
    {
        if ($this->socket === null) return;
        try {
            $this->command([['LOGOUT']]);
        } catch (RuntimeException) {
        }
        @fclose($this->socket);
        $this->socket = null;
    }

    /**
     * The account: EMBASSY_IMAP_USER / _PASS, else the Embassy's SMTP account, else SMTP_USER / _PASS
     * @return array{0: string, 1: string}
     */
    public static function credentials(Settings $settings): array
    {
        foreach (['EMBASSY_IMAP', 'EMBASSY_SMTP', 'SMTP'] as $prefix) {
            $user = $settings->get("{$prefix}_USER");
            if ($user !== '') return [$user, $settings->get("{$prefix}_PASS")];
        }
        return ['', ''];
    }

    // ── Protocol ──────────────────────────────────────────────────────────────────────────────────────────

    /**
     * An IMAP astring: a quoted string when it is printable ASCII, else a literal
     * @return list<array{0: string, 1?: string}>
     */
    private static function astring(string $value): array
    {
        if (preg_match('/^[\x20-\x7e]*$/', $value)) return [['"' . addcslashes($value, '\\"') . '"']];
        return [['', $value]];
    }

    /**
     * Sends one tagged command – parts: [text] or [text, literal] (the literal follows the text as {n}) – and reads
     * the reply up to its tagged line. A NO / BAD throws.
     * @param list<array{0: string, 1?: string}> $parts
     * @return array{lines: list<string>, literals: list<string>}
     */
    private function command(array $parts, ?string $logAs = null): array
    {
        $tag = 'A' . (++$this->tag);
        $buffer = "$tag ";
        foreach ($parts as $part) {
            $buffer .= $part[0];
            if (!array_key_exists(1, $part)) continue;
            $this->write($buffer . '{' . strlen($part[1]) . "}\r\n");
            $buffer = '';
            // the server asks for the literal with a continuation line ("+ …")
            $line = $this->line();
            if (!str_starts_with($line, '+')) throw new RuntimeException('IMAP: literal refused: ' . trim($line));
            $this->write($part[1]);
        }
        $this->write("$buffer\r\n");
        return $this->response($tag, $logAs ?? trim($buffer) ?: 'command');
    }

    /** @return array{lines: list<string>, literals: list<string>} */
    private function response(string $tag, string $after): array
    {
        $lines = [];
        $literals = [];
        while (true) {
            $line = $this->line();
            // a line ending in {n} is followed by n bytes of literal data, then the rest of the line
            while (preg_match('/\{(\d+)\}\r?\n$/', $line, $m)) {
                $literals[] = $this->read((int) $m[1]);
                $line = substr($line, 0, -strlen($m[0])) . '{literal}' . $this->line();
            }
            if (str_starts_with($line, "$tag ")) {
                if (!preg_match('/^\S+ OK\b/i', $line)) throw new RuntimeException("IMAP: $after failed: " . trim(substr($line, strlen($tag) + 1)));
                return ['lines' => $lines, 'literals' => $literals];
            }
            $lines[] = rtrim($line, "\r\n");
        }
    }

    private function line(): string
    {
        if ($this->socket === null) throw new RuntimeException('IMAP not connected');
        $line = fgets($this->socket);
        if ($line === false) {
            throw new RuntimeException(stream_get_meta_data($this->socket)['timed_out'] ? 'IMAP: timeout' : 'IMAP: connection closed');
        }
        return $line;
    }

    private function read(int $bytes): string
    {
        if ($this->socket === null) throw new RuntimeException('IMAP not connected');
        $data = '';
        while (($missing = $bytes - strlen($data)) > 0) {
            $chunk = fread($this->socket, min(65536, $missing));
            if ($chunk === false || ($chunk === '' && feof($this->socket))) throw new RuntimeException('IMAP: connection closed');
            if ($chunk === '' && stream_get_meta_data($this->socket)['timed_out']) throw new RuntimeException('IMAP: timeout');
            $data .= $chunk;
        }
        return $data;
    }

    private function write(string $data): void
    {
        while ($data !== '') {
            $written = $this->socket !== null ? fwrite($this->socket, $data) : false;
            if ($written === false || $written === 0) throw new RuntimeException('IMAP write failed');
            $data = substr($data, $written);
        }
    }
}
