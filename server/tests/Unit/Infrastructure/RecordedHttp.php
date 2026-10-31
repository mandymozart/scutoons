<?php
declare(strict_types=1);

namespace Scutoons\Tests\Infrastructure;

use Scutoons\Infrastructure\AssetStorage\HttpClient;

/** An HttpClient that answers from a list (status, body) and records every request – SoapKasApiTest (#201). */
final class RecordedHttp implements HttpClient
{
    /** @var list<array{url: string, headers: array<string, string>, body: string}> */
    public array $requests = [];

    /** @param list<array{int, string}> $answers */
    public function __construct(private array $answers)
    {
    }

    public function send(string $method, string $url, array $headers = [], ?string $body = null, ?string $bodyFile = null, ?string $sinkFile = null): array
    {
        $this->requests[] = ['url' => $url, 'headers' => $headers, 'body' => (string) $body];
        [$status, $answer] = array_shift($this->answers) ?? [500, ''];
        return ['status' => $status, 'body' => $answer, 'headers' => []];
    }
}
