<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use RuntimeException;
use Scutoons\Infrastructure\AssetStorage\HttpClient;

/**
 * A stand-in for the paid providers (#121) – never a real paid call: Luma's Agents API on https://luma.test, Runway's
 * developer API on https://runway.test, result links on https://files.test, the ECB's daily XML on https://ecb.test.
 * Speaks their documented wire format (the generators' docblocks); every request is recorded. Tests set the jobs'
 * states and make a start fail (`startStatus`, 0 = no answer).
 */
final class FakeProviders implements HttpClient
{
    public const LUMA_KEY = 'luma-test-key';
    public const RUNWAY_KEY = 'runway-test-key';
    public const PNG = "\x89PNG\r\n\x1a\n\0\0\0\rIHDR-result";

    /** @var list<array{method: string, url: string, headers: array<string, string>, body: mixed}> */
    public array $requests = [];
    /** @var array<string, array<string, mixed>> Luma generation id → its GET answer */
    public array $luma = [];
    /** @var array<string, array<string, mixed>> Runway task id → its GET answer */
    public array $runway = [];
    /** the next start's HTTP status (200 = created; 0 = no answer) */
    public int $startStatus = 200;
    public string $ecbXml = "<gesmes:Envelope><Cube><Cube time='2026-10-20'><Cube currency='USD' rate='1.25'/><Cube currency='JPY' rate='170.1'/></Cube></Cube></gesmes:Envelope>";
    private int $next = 0;

    public function send(string $method, string $url, array $headers = [], ?string $body = null, ?string $bodyFile = null, ?string $sinkFile = null): array
    {
        $json = $body !== null ? json_decode($body, true) : null;
        $this->requests[] = ['method' => $method, 'url' => $url, 'headers' => $headers, 'body' => $json];
        $parts = parse_url($url);
        $host = (string) ($parts['host'] ?? '');
        $path = (string) ($parts['path'] ?? '');
        $answer = match ($host) {
            'luma.test' => $this->lumaApi($method, $path, $headers),
            'runway.test' => $this->runwayApi($method, $path, $headers),
            'ecb.test' => self::answer(200, $this->ecbXml, 'text/xml'),
            'files.test' => str_contains($path, 'expired') ? self::answer(403, 'AccessDenied', 'text/plain') : self::answer(200, self::PNG, 'image/png'),
            default => throw new RuntimeException("Could not resolve host: $host"),
        };
        if ($sinkFile !== null) {
            file_put_contents($sinkFile, $answer['body']);
            $answer['body'] = '';
        }
        return $answer;
    }

    /** @return list<string> the requests to one host (method + path) */
    public function calls(string $host): array
    {
        return array_values(array_map(fn(array $r) => $r['method'] . ' ' . parse_url($r['url'], PHP_URL_PATH),
            array_filter($this->requests, fn(array $r) => parse_url($r['url'], PHP_URL_HOST) === $host)));
    }

    /** @return array<mixed> the JSON body of the last request to a path ([] = none) */
    public function lastBody(string $path): array
    {
        foreach (array_reverse($this->requests) as $r) {
            if (parse_url($r['url'], PHP_URL_PATH) === $path) return is_array($r['body']) ? $r['body'] : [];
        }
        return [];
    }

    /**
     * @param array<string, string> $headers
     * @return array{status: int, body: string, headers: array<string, string>}
     */
    private function lumaApi(string $method, string $path, array $headers): array
    {
        if (($headers['Authorization'] ?? '') !== 'Bearer ' . self::LUMA_KEY) return self::answer(401, '{"detail":"Invalid API key"}');
        if ($method === 'POST' && $path === '/v1/generations') {
            if ($this->startStatus === 0) throw new RuntimeException('Operation timed out after 30000 milliseconds');
            if ($this->startStatus !== 200) return self::answer($this->startStatus, '{"detail":"prompt rejected by moderation"}');
            $id = 'luma-' . ++$this->next;
            $this->luma[$id] = ['id' => $id, 'state' => 'queued', 'output' => [], 'failure_code' => null, 'failure_reason' => null];
            return self::answer(201, (string) json_encode($this->luma[$id]));
        }
        if ($method === 'GET' && preg_match('#^/v1/generations/([\w-]+)$#', $path, $m) && isset($this->luma[$m[1]])) return self::answer(200, (string) json_encode($this->luma[$m[1]]));
        return self::answer(404, '{"detail":"Not found"}');
    }

    /**
     * @param array<string, string> $headers
     * @return array{status: int, body: string, headers: array<string, string>}
     */
    private function runwayApi(string $method, string $path, array $headers): array
    {
        if (($headers['Authorization'] ?? '') !== 'Bearer ' . self::RUNWAY_KEY || ($headers['X-Runway-Version'] ?? '') !== '2024-11-06') return self::answer(401, '{"error":"Unauthorized"}');
        if ($method === 'POST' && in_array($path, ['/v1/image_to_video', '/v1/video_to_video'], true)) {
            if ($this->startStatus === 0) throw new RuntimeException('Connection reset');
            if ($this->startStatus !== 200) return self::answer($this->startStatus, '{"error":"Invalid input","issues":[]}');
            $id = 'rw-' . ++$this->next;
            $this->runway[$id] = ['id' => $id, 'status' => 'PENDING'];
            return self::answer(200, (string) json_encode(['id' => $id]));
        }
        if (preg_match('#^/v1/tasks/([\w-]+)$#', $path, $m) && isset($this->runway[$m[1]])) {
            if ($method === 'DELETE') {
                $this->runway[$m[1]]['status'] = 'CANCELLED';
                return self::answer(204, '');
            }
            return self::answer(200, (string) json_encode($this->runway[$m[1]]));
        }
        return self::answer(404, '{"error":"Not found"}');
    }

    /** @return array{status: int, body: string, headers: array<string, string>} */
    private static function answer(int $status, string $body, string $type = 'application/json'): array
    {
        return ['status' => $status, 'body' => $body, 'headers' => ['content-type' => $type]];
    }
}
