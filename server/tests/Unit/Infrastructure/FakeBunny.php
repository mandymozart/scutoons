<?php
declare(strict_types=1);

namespace Scutoons\Tests\Infrastructure;

use Scutoons\Infrastructure\AssetStorage\HttpClient;

/**
 * An in-memory bunny.net (Storage API + the Core API's purge / statistics) speaking the documented wire format, so
 * BunnyStorage and everything above it is tested without the owner's zone. Records every request (no secrets in
 * assertions: tests only check that the AccessKey header is the configured one).
 */
final class FakeBunny implements HttpClient
{
    /** @var array<string, string> key → bytes */
    public array $files = [];
    /** @var list<array{method: string, url: string, headers: array<string, string>}> */
    public array $requests = [];
    /** @var list<string> */
    public array $purged = [];

    public function __construct(public string $zone = 'zone', public string $password = 'zone-password', public string $apiKey = 'api-key', public string $readonlyPassword = 'read-only-password')
    {
    }

    public function send(string $method, string $url, array $headers = [], ?string $body = null, ?string $bodyFile = null, ?string $sinkFile = null): array
    {
        $this->requests[] = ['method' => $method, 'url' => $url, 'headers' => $headers];
        $parts = parse_url($url);
        $host = (string) ($parts['host'] ?? '');
        $path = rawurldecode((string) ($parts['path'] ?? ''));
        parse_str((string) ($parts['query'] ?? ''), $query);

        if ($host === 'api.bunny.net') {
            if (($headers['AccessKey'] ?? '') !== $this->apiKey) return self::answer(401, '');
            if ($method === 'POST' && $path === '/purge') {
                $this->purged[] = is_string($query['url'] ?? null) ? $query['url'] : '';
                return self::answer(200, '');
            }
            if ($method === 'GET' && $path === '/statistics') {
                return self::answer(200, (string) json_encode(['TotalBandwidthUsed' => 123456789, 'TotalRequestsServed' => 4321, 'CacheHitRate' => 97.5, 'BandwidthUsedChart' => []]));
            }
            return self::answer(404, '');
        }
        if ($host === 'logging.bunnycdn.com') {
            if (($headers['AccessKey'] ?? '') !== $this->apiKey) return self::answer(401, '');
            return self::answer(200, implode("\n", [
                'HIT|200|1790000000000|1000|1|1.2.3.4|-|https://cdn.example/catpope/videos/a.720p.mp4|DE|ua|x|DE',
                'MISS|206|1790000001000|500|1|1.2.3.5|-|https://cdn.example/catpope/videos/a.720p.mp4?v=1|DE|ua|y|AT',
                'HIT|404|1790000002000|10|1|1.2.3.6|-|https://cdn.example/nope.png|DE|ua|z|DE',
            ]));
        }
        if ($host === 'scutoons.b-cdn.net' || $host === 'cdn.example') {
            $key = ltrim($path, '/');
            return isset($this->files[$key]) ? ['status' => 200, 'body' => '', 'headers' => ['content-length' => (string) strlen($this->files[$key]), 'cdn-cache' => 'HIT']] : self::answer(404, '');
        }
        $access = $headers['AccessKey'] ?? '';
        $reading = $method === 'GET';
        if ($access !== $this->password && !($reading && $access === $this->readonlyPassword)) return self::answer(401, '');
        $prefix = '/' . $this->zone . '/';
        if (!str_starts_with($path . '/', $prefix)) return self::answer(404, '');
        $key = substr($path, strlen($prefix));

        if ($method === 'PUT') {
            $bytes = $bodyFile !== null ? (string) file_get_contents($bodyFile) : (string) $body;
            if (isset($headers['Checksum']) && strtoupper(hash('sha256', $bytes)) !== $headers['Checksum']) return self::answer(400, 'checksum');
            $this->files[$key] = $bytes;
            return self::answer(201, '{"HttpCode":201,"Message":"File uploaded."}');
        }
        if ($method === 'DELETE') {
            if (!isset($this->files[$key])) return self::answer(404, '');
            unset($this->files[$key]);
            return self::answer(200, '');
        }
        if ($method === 'GET' && (str_ends_with($path, '/'))) {
            return self::answer(200, (string) json_encode($this->listing(rtrim($key, '/'))));
        }
        if ($method === 'GET') {
            if (!isset($this->files[$key])) return self::answer(404, '');
            if ($sinkFile !== null) {
                file_put_contents($sinkFile, $this->files[$key]);
                return self::answer(200, '');
            }
            return self::answer(200, $this->files[$key]);
        }
        return self::answer(405, '');
    }

    /** @return list<array<string, mixed>> */
    private function listing(string $dir): array
    {
        $out = [];
        $dirs = [];
        $base = $dir === '' ? '' : $dir . '/';
        foreach ($this->files as $key => $bytes) {
            if ($base !== '' && !str_starts_with($key, $base)) continue;
            $rest = substr($key, strlen($base));
            if (str_contains($rest, '/')) {
                $dirs[substr($rest, 0, (int) strpos($rest, '/'))] = true;
                continue;
            }
            $out[] = ['Guid' => md5($key), 'StorageZoneName' => $this->zone, 'Path' => '/' . $this->zone . '/' . $base, 'ObjectName' => $rest,
                'Length' => strlen($bytes), 'LastChanged' => '2026-10-02T10:00:00.000', 'IsDirectory' => false, 'ServerId' => 1,
                'Checksum' => strtoupper(hash('sha256', $bytes)), 'ContentType' => '', 'DateCreated' => '2026-10-01T10:00:00.000'];
        }
        foreach (array_keys($dirs) as $name) {
            $out[] = ['Guid' => md5($base . $name), 'Path' => '/' . $this->zone . '/' . $base, 'ObjectName' => $name, 'Length' => 0, 'IsDirectory' => true, 'LastChanged' => '2026-10-02T10:00:00.000'];
        }
        return $out;
    }

    /** @return array{status: int, body: string, headers: array<string, string>} */
    private static function answer(int $status, string $body): array
    {
        return ['status' => $status, 'body' => $body, 'headers' => []];
    }
}
