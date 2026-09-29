<?php
declare(strict_types=1);

namespace Qr;

/** An HTTP answer: status, headers (in order, repeatable) and body */
final class Response
{
    /** @param list<array{0: string, 1: string}> $headers */
    public function __construct(public readonly int $status, public readonly array $headers, public readonly string $body)
    {
    }

    public static function html(int $status, string $body): self
    {
        return new self($status, [['Content-Type', 'text/html; charset=UTF-8'], ['Cache-Control', 'no-store']], $body);
    }

    public static function redirect(string $location): self
    {
        return new self(302, [['Location', $location], ['Content-Type', 'text/html; charset=UTF-8'], ['Cache-Control', 'no-store']], '');
    }

    public static function json(int $status, string $json): self
    {
        return new self($status, [['Content-Type', 'application/json; charset=utf-8'], ['Cache-Control', 'no-store']], $json);
    }

    public function header(string $name): ?string
    {
        foreach ($this->headers as [$n, $v]) {
            if (strcasecmp($n, $name) === 0) return $v;
        }
        return null;
    }

    public function withHeader(string $name, string $value): self
    {
        $headers = array_values(array_filter($this->headers, static fn(array $h): bool => strcasecmp($h[0], $name) !== 0));
        $headers[] = [$name, $value];
        return new self($this->status, $headers, $this->body);
    }

    public function send(bool $withBody = true): void
    {
        // No PHP defaults on top of what we (or the API) chose: no text/html fallback type, no X-Powered-By
        ini_set('default_mimetype', '');
        header_remove('X-Powered-By');
        http_response_code($this->status);
        foreach ($this->headers as [$name, $value]) header($name . ': ' . $value, false);
        if ($withBody) echo $this->body;
    }
}
