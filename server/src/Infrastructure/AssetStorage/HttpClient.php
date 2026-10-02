<?php
declare(strict_types=1);

namespace Scutoons\Infrastructure\AssetStorage;

/**
 * The few HTTP calls the remote storage drivers make (bunny.net Storage + Core API). An interface so the tests run
 * the drivers against a fake (tests/Unit/Infrastructure/FakeBunny.php) – never against the owner's zone.
 */
interface HttpClient
{
    /**
     * @param array<string, string> $headers
     * @param string|null $body raw request body
     * @param string|null $bodyFile a local file streamed as the body (uploads; wins over $body)
     * @param string|null $sinkFile write the response body into this local file (downloads) – the returned body is ''
     * @return array{status: int, body: string, headers: array<string, string>}
     */
    public function send(string $method, string $url, array $headers = [], ?string $body = null, ?string $bodyFile = null, ?string $sinkFile = null): array;
}
