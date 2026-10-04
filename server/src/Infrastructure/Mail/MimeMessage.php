<?php
declare(strict_types=1);

namespace Scutoons\Infrastructure\Mail;

use Throwable;

/**
 * A small MIME reader for incoming mail (the Embassy's inbox sync): headers (unfolded, RFC 2047 encoded-words decoded),
 * multipart recursion, the first text/plain part (else the first text/html part as plain text), Content-Transfer-
 * Encoding quoted-printable / base64 / 7bit / 8bit, the part's charset converted to UTF-8. Attachments and attached
 * messages (message/rfc822) are never read. No extension needed beyond mbstring (iconv when there).
 */
final class MimeMessage
{
    /** Multipart nesting deeper than this is not followed (a broken or hostile mail) */
    private const MAX_DEPTH = 8;

    /**
     * @param array<string, list<string>> $headers lower-case name → the raw (unfolded, undecoded) values, in order
     */
    private function __construct(private readonly array $headers, private readonly string $body)
    {
    }

    public static function parse(string $raw): self
    {
        [$headers, $body] = self::split($raw);
        return new self($headers, $body);
    }

    /** The first value of a header, encoded-words decoded, trimmed ('' = not there) */
    public function header(string $name): string
    {
        $value = $this->headers[strtolower($name)][0] ?? null;
        return $value === null ? '' : trim(self::decodeHeader($value));
    }

    /** The first value of a header as sent (unfolded, not decoded) */
    public function rawHeader(string $name): string
    {
        return trim($this->headers[strtolower($name)][0] ?? '');
    }

    public function has(string $name): bool
    {
        return isset($this->headers[strtolower($name)]);
    }

    /**
     * The message ids in a header (Message-ID, In-Reply-To, References) without the angle brackets
     * @return list<string>
     */
    public function ids(string $name): array
    {
        $ids = [];
        foreach ($this->headers[strtolower($name)] ?? [] as $value) {
            preg_match_all('/<([^<>\s]+)>/', $value, $m);
            foreach ($m[1] as $id) $ids[] = $id;
        }
        return array_values(array_unique($ids));
    }

    /** The address of the first mailbox in an address header (From, Reply-To), lower-cased ('' = none) */
    public function address(string $name): string
    {
        $value = $this->rawHeader($name);
        if (preg_match('/<([^<>\s@]+@[^<>\s]+)>/', $value, $m)) return strtolower($m[1]);
        if (preg_match('/([^\s<>"(),;:]+@[^\s<>"(),;:]+)/', $value, $m)) return strtolower($m[1]);
        return '';
    }

    /** The readable text: the first text/plain part, else the first text/html part as plain text ('' = none) */
    public function text(): string
    {
        $found = $this->find($this->headers, $this->body, 0);
        if ($found['plain'] !== null) return self::tidy($found['plain']);
        if ($found['html'] !== null) return self::tidy(self::htmlToText($found['html']));
        return '';
    }

    // ── Internals ─────────────────────────────────────────────────────────────────────────────────────────

    /**
     * Searches one entity (the message or a part) depth first
     * @param array<string, list<string>> $headers
     * @return array{plain: ?string, html: ?string}
     */
    private function find(array $headers, string $body, int $depth): array
    {
        $found = ['plain' => null, 'html' => null];
        [$type, $params] = self::contentType($headers['content-type'][0] ?? 'text/plain');
        $disposition = strtolower(trim(explode(';', $headers['content-disposition'][0] ?? '')[0]));
        if ($disposition === 'attachment') return $found;
        if (str_starts_with($type, 'multipart/')) {
            $boundary = $params['boundary'] ?? '';
            if ($boundary === '' || $depth >= self::MAX_DEPTH) return $found;
            foreach (self::parts($body, $boundary) as $part) {
                [$partHeaders, $partBody] = self::split($part);
                $inner = $this->find($partHeaders, $partBody, $depth + 1);
                $found['plain'] ??= $inner['plain'];
                $found['html'] ??= $inner['html'];
                if ($found['plain'] !== null) break;
            }
            return $found;
        }
        if ($type !== 'text/plain' && $type !== 'text/html') return $found;
        $text = self::charset(self::decodeBody($body, $headers['content-transfer-encoding'][0] ?? ''), $params['charset'] ?? '');
        $found[$type === 'text/plain' ? 'plain' : 'html'] = $text;
        return $found;
    }

    /**
     * Header block and body; header lines unfolded
     * @return array{0: array<string, list<string>>, 1: string}
     */
    private static function split(string $raw): array
    {
        $raw = str_replace(["\r\n", "\r"], "\n", $raw);
        $end = strpos($raw, "\n\n");
        if (str_starts_with($raw, "\n")) {
            [$head, $body] = ['', substr($raw, 1)];
        } else {
            [$head, $body] = $end === false ? [$raw, ''] : [substr($raw, 0, $end), substr($raw, $end + 2)];
        }
        $head = (string) preg_replace('/\n[ \t]+/', ' ', $head);
        $headers = [];
        foreach (explode("\n", $head) as $line) {
            $colon = strpos($line, ':');
            if ($colon === false || $colon === 0) continue;
            $name = strtolower(trim(substr($line, 0, $colon)));
            if ($name === '' || preg_match('/\s/', $name)) continue;
            $headers[$name][] = trim(substr($line, $colon + 1));
        }
        return [$headers, $body];
    }

    /** @return list<string> the parts between the boundary lines */
    private static function parts(string $body, string $boundary): array
    {
        $parts = [];
        $current = null;
        foreach (explode("\n", $body) as $line) {
            $trimmed = rtrim($line);
            if ($trimmed === "--$boundary--") break;
            if ($trimmed === "--$boundary") {
                if ($current !== null) $parts[] = implode("\n", $current);
                $current = [];
                continue;
            }
            if ($current !== null) $current[] = $line;
        }
        if ($current !== null) $parts[] = implode("\n", $current);
        return $parts;
    }

    /**
     * "text/plain; charset="utf-8"" → ['text/plain', ['charset' => 'utf-8']]
     * @return array{0: string, 1: array<string, string>}
     */
    private static function contentType(string $value): array
    {
        $pieces = explode(';', $value, 2);
        $type = strtolower(trim($pieces[0]));
        $params = [];
        preg_match_all('/([\w*-]+)\s*=\s*("((?:[^"\\\\]|\\\\.)*)"|[^;\s]+)/', $pieces[1] ?? '', $m, PREG_SET_ORDER);
        foreach ($m as $param) {
            $params[strtolower($param[1])] = isset($param[3]) && $param[2][0] === '"' ? stripslashes($param[3]) : $param[2];
        }
        return [$type !== '' ? $type : 'text/plain', $params];
    }

    private static function decodeBody(string $body, string $encoding): string
    {
        return match (strtolower(trim($encoding))) {
            'quoted-printable' => quoted_printable_decode($body),
            'base64' => (string) base64_decode((string) preg_replace('/[^A-Za-z0-9+\/=]/', '', $body)),
            default => $body,
        };
    }

    /** Text in $charset → UTF-8 (unknown charsets: kept, invalid bytes replaced) */
    private static function charset(string $text, string $charset): string
    {
        $charset = strtolower(trim($charset, " \t\"'"));
        if ($charset !== '' && !in_array($charset, ['utf-8', 'utf8', 'us-ascii', 'ascii'], true)) {
            // windows-1252 is what mail clients mean by iso-8859-1 (the HTML5 rule): a superset for the printable range
            if (in_array($charset, ['iso-8859-1', 'latin1', 'latin-1'], true)) $charset = 'windows-1252';
            $converted = null;
            try {
                $converted = mb_convert_encoding($text, 'UTF-8', $charset);
            } catch (Throwable) {
                if (function_exists('iconv')) {
                    $iconv = @iconv($charset, 'UTF-8//IGNORE', $text);
                    if (is_string($iconv)) $converted = $iconv;
                }
            }
            if (is_string($converted)) $text = $converted;
        }
        return mb_scrub($text, 'UTF-8');
    }

    /** RFC 2047: =?charset?B|Q?…?= (adjacent encoded-words join without the space between them) */
    public static function decodeHeader(string $value): string
    {
        $value = (string) preg_replace('/(=\?[^?\s]+\?[bBqQ]\?[^?\s]*\?=)\s+(?==\?)/', '$1', $value);
        $decoded = preg_replace_callback('/=\?([^?\s]+)\?([bBqQ])\?([^?\s]*)\?=/', static function (array $m): string {
            $charset = (string) preg_replace('/\*.*$/', '', $m[1]); // RFC 2231 language suffix (utf-8*de)
            $bytes = strtoupper($m[2]) === 'B'
                ? (string) base64_decode($m[3])
                : quoted_printable_decode(str_replace('_', ' ', $m[3]));
            return self::charset($bytes, $charset);
        }, $value);
        return mb_scrub((string) $decoded, 'UTF-8');
    }

    /** HTML → plain text: blocks become line breaks, tags go, entities are decoded */
    public static function htmlToText(string $html): string
    {
        $html = (string) preg_replace('#<(head|style|script|title)\b[^>]*>.*?</\1\s*>#is', '', $html);
        $html = (string) preg_replace('#<br\s*/?>#i', "\n", $html);
        $html = (string) preg_replace('#</(p|div|tr|h[1-6]|blockquote|table|ul|ol)\s*>#i', "\n", $html);
        $html = (string) preg_replace('#<(p|div|blockquote|table|ul|ol)\b[^>]*>#i', "\n", $html);
        $html = (string) preg_replace('#<li\b[^>]*>#i', "\n- ", $html);
        $text = html_entity_decode(strip_tags($html), ENT_QUOTES | ENT_HTML5, 'UTF-8');
        $text = str_replace("\u{a0}", ' ', $text);
        return (string) preg_replace('/[ \t]+/', ' ', $text);
    }

    /** Line breaks to \n, trailing spaces off, at most one empty line in a row */
    private static function tidy(string $text): string
    {
        $text = str_replace(["\r\n", "\r"], "\n", $text);
        $text = (string) preg_replace('/[ \t]+\n/', "\n", $text);
        $text = (string) preg_replace("/\n{3,}/", "\n\n", $text);
        return trim($text);
    }
}
