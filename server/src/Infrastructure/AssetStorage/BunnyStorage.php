<?php
declare(strict_types=1);

namespace Scutoons\Infrastructure\AssetStorage;

use RuntimeException;

/**
 * A bunny.net Storage Zone (+ its Pull Zone) – agents/ASSETS.md "bunny.net". Built against the official reference
 * (bunny.net/docs/api-reference/storage = the "bunny.net Edge Storage API" OpenAPI 1.0.0, fetched 2026-10-02;
 * bunny.net/docs/storage/http for the region endpoints; api-reference/core for purge + statistics):
 *   Storage API  https://{endpoint}/{zone}/{path}   endpoint = the zone's region hostname (BUNNY_STORAGE_ENDPOINT:
 *                                                   storage.bunnycdn.com, uk. / ny. / la. / sg. / se. / br. / jh. / syd.)
 *     header AccessKey: <a storage zone password> – the READ-ONLY password for every GET (list, download, stat), the
 *     read/write password only for PUT / DELETE (BUNNY_STORAGE_READONLY_PASSWORD / BUNNY_STORAGE_PASSWORD)
 *     PUT    …/{path}        raw body, `Checksum: <SHA-256 hex, UPPER CASE>` (the server rejects a mismatch) → 201, 400
 *     GET    …/{path}        the file → 200, 404
 *     GET    …/{dir}/        (trailing slash) JSON array of StorageObject: Guid, StorageZoneName, Path ("/{zone}/{dir}/"),
 *                            ObjectName, Length, LastChanged, IsDirectory, ServerId, UserId, DateCreated, StorageZoneId
 *                            (+ Checksum / ContentType when present – used if there, never required)
 *     DELETE …/{path}        a file → 200, 400 (a directory would go recursively – this driver never deletes directories)
 *   Core API     https://api.bunny.net, header AccessKey: <the account API key> (BUNNY_API_KEY, optional)
 *     POST /purge?url=<public url>&async=false        drop one URL from the edge caches
 *     GET  /statistics?dateFrom&dateTo&pullZone        TotalBandwidthUsed, TotalRequestsServed, CacheHitRate, charts
 *   Logging API  https://logging.bunnycdn.com/{MM-DD-YY}/{pullZoneId}.log (API key) – raw per-request lines
 *     cache|status|timestamp|bytes|zone|ip|referer|url|edge|agent|id|country → per-file traffic
 * Secrets never leave this class: errors name the method, the key and the status, never a header.
 * `writePrefix` (BUNNY_WRITE_PREFIX) restricts every write / delete to keys below it (e.g. "_test/" while trying the
 * driver against the production zone); '' = no restriction.
 */
final class BunnyStorage implements AssetStorage
{
    public const API = 'https://api.bunny.net';
    public const LOGGING = 'https://logging.bunnycdn.com';

    public function __construct(private readonly HttpClient $http, private readonly BunnyConfig $config)
    {
        if (!$config->usable()) throw new RuntimeException('bunny.net storage is not configured (BUNNY_STORAGE_ZONE + BUNNY_STORAGE_READONLY_PASSWORD / BUNNY_STORAGE_PASSWORD).');
    }

    public function name(): string
    {
        return 'bunny';
    }

    public function config(): BunnyConfig
    {
        return $this->config;
    }

    public function describe(): string
    {
        $mode = $this->config->password === '' ? ', read-only' : ($this->config->writePrefix !== '' ? ", writes only below {$this->config->writePrefix}" : '');
        return "bunny.net storage zone {$this->config->zone} at {$this->host()}$mode";
    }

    public function host(): string
    {
        return $this->config->endpoint !== '' ? $this->config->endpoint : BunnyConfig::DEFAULT_ENDPOINT;
    }

    public function canWrite(): bool
    {
        return $this->config->password !== '';
    }

    public function hasApiKey(): bool
    {
        return $this->config->apiKey !== '';
    }

    public function pullZoneId(): string
    {
        return $this->config->pullZoneId;
    }

    private function url(string $key, bool $directory = false): string
    {
        $segments = $key === '' ? [] : array_map('rawurlencode', explode('/', rtrim($key, '/')));
        $path = implode('/', [rawurlencode($this->config->zone), ...$segments]);
        return 'https://' . $this->host() . '/' . $path . ($directory ? '/' : '');
    }

    /** @return array<string, string> the read-only password for reads (the read/write one when there is none) */
    private function readAuth(): array
    {
        return ['AccessKey' => $this->config->readonlyPassword !== '' ? $this->config->readonlyPassword : $this->config->password];
    }

    /** @return array<string, string> */
    private function writeAuth(string $key): array
    {
        if ($this->config->password === '') throw new RuntimeException('The bunny.net zone is read-only for the asset layer (no BUNNY_STORAGE_PASSWORD).');
        if ($this->config->writePrefix !== '' && !str_starts_with($key, $this->config->writePrefix)) {
            throw new RuntimeException("Refused: writes on the bunny.net zone are limited to {$this->config->writePrefix} (BUNNY_WRITE_PREFIX); $key is outside.");
        }
        return ['AccessKey' => $this->config->password];
    }

    /** Why a status likely happened – for the check command and error messages (never contains a secret) */
    public static function hint(int $status): string
    {
        return match ($status) {
            401 => 'unauthorized – wrong password for this zone (the zone → FTP & API Access → Password / Read-only password; not the account API key)',
            404 => 'not found – wrong zone name or wrong region endpoint (BUNNY_STORAGE_ENDPOINT = the zone → FTP & API Access → Hostname)',
            0 => 'no connection – check the endpoint host name',
            default => '',
        };
    }

    /** @return list<array<string, mixed>> the raw objects of one directory ('' = the zone root) */
    public function listDirectory(string $dir): array
    {
        $dir = StorageKey::normalize($dir, true);
        $response = $this->http->send('GET', $this->url($dir, true), $this->readAuth() + ['Accept' => 'application/json']);
        // a missing sub-folder answers 404; the zone root never does (a 404 there = wrong zone / endpoint)
        if ($response['status'] === 404 && $dir !== '') return [];
        if ($response['status'] !== 200) throw new RuntimeException("bunny.net list $dir/ failed: HTTP {$response['status']} " . self::hint($response['status']));
        $data = json_decode($response['body'], true);
        if (!is_array($data)) throw new RuntimeException("bunny.net list $dir/: unexpected answer");
        return array_values(array_filter($data, 'is_array'));
    }

    /** @param array<string, mixed> $object */
    private function keyOf(array $object): string
    {
        // Path = "/<zone>/<dir>/", ObjectName = the file name
        $path = (string) ($object['Path'] ?? '');
        $prefix = '/' . $this->config->zone . '/';
        if (str_starts_with($path, $prefix)) $path = substr($path, strlen($prefix));
        else $path = ltrim($path, '/');
        return ltrim($path . (string) ($object['ObjectName'] ?? ''), '/');
    }

    /** @param array<string, mixed> $object */
    private function fileOf(array $object): StoredFile
    {
        $key = $this->keyOf($object);
        $changed = (string) ($object['LastChanged'] ?? '');
        $checksum = isset($object['Checksum']) && is_string($object['Checksum']) && $object['Checksum'] !== '' ? strtolower($object['Checksum']) : null;
        $mime = isset($object['ContentType']) && is_string($object['ContentType']) && $object['ContentType'] !== '' ? $object['ContentType'] : StorageKey::mime($key);
        return new StoredFile($key, (int) ($object['Length'] ?? 0), $changed !== '' ? gmdate('c', (int) strtotime($changed . (preg_match('/Z|[+-]\d\d:?\d\d$/', $changed) ? '' : 'Z'))) : '', $checksum, $mime);
    }

    public function list(string $prefix = ''): iterable
    {
        $prefix = StorageKey::normalize($prefix, true);
        $queue = [rtrim($prefix, '/')];
        while ($queue) {
            $dir = array_shift($queue);
            foreach ($this->listDirectory($dir) as $object) {
                if (!empty($object['IsDirectory'])) {
                    $queue[] = $this->keyOf($object);
                    continue;
                }
                yield $this->fileOf($object);
            }
        }
    }

    public function stat(string $key): ?StoredFile
    {
        $key = StorageKey::normalize($key);
        $dir = StorageKey::dirname($key);
        $name = StorageKey::basename($key);
        foreach ($this->listDirectory($dir) as $object) {
            if (empty($object['IsDirectory']) && (string) ($object['ObjectName'] ?? '') === $name) return $this->fileOf($object);
        }
        return null;
    }

    public function download(string $key, string $localFile): void
    {
        $key = StorageKey::normalize($key);
        $response = $this->http->send('GET', $this->url($key), $this->readAuth(), null, null, $localFile);
        if ($response['status'] !== 200) {
            @unlink($localFile);
            throw new RuntimeException("bunny.net download $key failed: HTTP {$response['status']} " . self::hint($response['status']));
        }
    }

    public function upload(string $key, string $localFile, ?string $mime = null): StoredFile
    {
        $key = StorageKey::normalize($key);
        $auth = $this->writeAuth($key);
        $hash = hash_file('sha256', $localFile);
        if ($hash === false) throw new RuntimeException('Cannot read the upload file.');
        $headers = $auth + ['Content-Type' => $mime ?? StorageKey::mime($key), 'Checksum' => strtoupper($hash)];
        $response = $this->http->send('PUT', $this->url($key), $headers, null, $localFile);
        if ($response['status'] !== 201 && $response['status'] !== 200) throw new RuntimeException("bunny.net upload $key failed: HTTP {$response['status']} " . self::hint($response['status']));
        return new StoredFile($key, (int) filesize($localFile), gmdate('c'), $hash, $mime ?? StorageKey::mime($key));
    }

    public function delete(string $key): void
    {
        $key = StorageKey::normalize($key);
        if (str_ends_with($key, '/')) throw new RuntimeException('Refused: the asset layer never deletes directories.');
        $response = $this->http->send('DELETE', $this->url($key), $this->writeAuth($key));
        if ($response['status'] !== 200 && $response['status'] !== 404) throw new RuntimeException("bunny.net delete $key failed: HTTP {$response['status']}");
    }

    public function copy(string $from, string $to): StoredFile
    {
        // the Storage API has no server-side copy: through a temp file
        $tmp = tempnam(sys_get_temp_dir(), 'bunny-copy-');
        if ($tmp === false) throw new RuntimeException('No temp file.');
        try {
            $this->download($from, $tmp);
            return $this->upload($to, $tmp);
        } finally {
            @unlink($tmp);
        }
    }

    public function publicUrl(string $key): string
    {
        return $this->config->publicBase === '' ? '' : $this->config->publicBase . '/' . implode('/', array_map('rawurlencode', explode('/', StorageKey::normalize($key))));
    }

    public function purge(string $key): bool
    {
        $url = $this->publicUrl($key);
        if ($this->config->apiKey === '' || $url === '') return false;
        $response = $this->http->send('POST', self::API . '/purge?url=' . rawurlencode($url) . '&async=false', ['AccessKey' => $this->config->apiKey]);
        return $response['status'] >= 200 && $response['status'] < 300;
    }

    /**
     * The pull zone's settings (Core API GET /pullzone/{id}, read-only): Name, OriginUrl, OriginType, StorageZoneId,
     * Hostnames [{ Value, IsSystemHostname, HasCertificate }] … Null without an API key or a numeric pull zone id.
     * @return array<string, mixed>|null
     */
    public function pullZone(): ?array
    {
        if ($this->config->apiKey === '' || $this->config->pullZoneId === '') return null;
        $response = $this->http->send('GET', self::API . '/pullzone/' . rawurlencode($this->config->pullZoneId), ['AccessKey' => $this->config->apiKey, 'Accept' => 'application/json']);
        if ($response['status'] !== 200) throw new RuntimeException("bunny.net pull zone {$this->config->pullZoneId} failed: HTTP {$response['status']}" . ($response['status'] === 401 ? ' – the account API key is wrong (account → API key)' : ''));
        $data = json_decode($response['body'], true);
        return is_array($data) ? $data : null;
    }

    /**
     * Pull-zone statistics (bandwidth, requests, cache hit rate) of a period; null without an API key.
     * @return array<string, mixed>|null
     */
    public function statistics(string $from, string $to, bool $hourly = false): ?array
    {
        if ($this->config->apiKey === '') return null;
        $query = http_build_query(array_filter([
            'dateFrom' => $from,
            'dateTo' => $to,
            'pullZone' => $this->config->pullZoneId !== '' ? $this->config->pullZoneId : null,
            'hourly' => $hourly ? 'true' : null,
        ], fn($v) => $v !== null));
        $response = $this->http->send('GET', self::API . '/statistics?' . $query, ['AccessKey' => $this->config->apiKey, 'Accept' => 'application/json']);
        if ($response['status'] !== 200) throw new RuntimeException("bunny.net statistics failed: HTTP {$response['status']}");
        $data = json_decode($response['body'], true);
        return is_array($data) ? $data : null;
    }

    /**
     * The raw request log of one day (Logging API v1, the pull zone's logging must be on): per line
     * cache|status|timestamp|bytes|zone|ip|referer|url|edge|agent|id|country. Null without an API key / pull zone id.
     */
    public function requestLog(string $day): ?string
    {
        if ($this->config->apiKey === '' || $this->config->pullZoneId === '') return null;
        $date = gmdate('m-d-y', (int) strtotime($day . ' 00:00:00 UTC'));
        $response = $this->http->send('GET', self::LOGGING . '/' . $date . '/' . rawurlencode($this->config->pullZoneId) . '.log', ['AccessKey' => $this->config->apiKey]);
        if ($response['status'] === 404) return '';
        if ($response['status'] !== 200) throw new RuntimeException("bunny.net request log $day failed: HTTP {$response['status']}");
        return $response['body'];
    }

    /**
     * HEAD of the public URL (through the pull zone) – does the CDN serve the file? No credentials involved.
     * @return array{url: string, status: int, size: ?int, cache: ?string}
     */
    public function headPublic(string $key): array
    {
        $url = $this->publicUrl($key);
        if ($url === '') return ['url' => '', 'status' => 0, 'size' => null, 'cache' => null];
        $response = $this->http->send('HEAD', $url);
        $headers = $response['headers'];
        return ['url' => $url, 'status' => $response['status'], 'size' => isset($headers['content-length']) ? (int) $headers['content-length'] : null,
            'cache' => $headers['cdn-cache'] ?? $headers['x-cache'] ?? null];
    }
}
