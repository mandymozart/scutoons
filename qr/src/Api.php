<?php
declare(strict_types=1);

namespace Qr;

/**
 * The two API calls of the landing page, with the cache:
 *   GET /api/code/?c=CODE[&scan=1]  public info about a code (scan=1: the API logs the scan – trusted proxy only)
 *   GET /api/items/                 the item catalog (names, descriptions, icons)
 * Successful code lookups are cached (CODE_CACHE_TTL fresh, CODE_CACHE_KEEP for the fallback page); the catalog
 * for ITEMS_CACHE_TTL (older copies are used while the API is down).
 */
final class Api
{
    public function __construct(private readonly Config $config, private readonly Http $http, private readonly Cache $cache)
    {
    }

    /**
     * Always asks the API (a scan must be counted and the rate limit applied), unless $preferCache and a fresh cached
     * answer exists (skip=1 redirects).
     * @return array{status: int, body: array<string, mixed>, retry_after: ?string, cached: bool}|null null = API down
     */
    public function code(string $code, bool $scan, bool $preferCache = false): ?array
    {
        if ($preferCache) {
            $cached = $this->cachedCode($code, $this->config->int('CODE_CACHE_TTL'));
            if ($cached !== null) return ['status' => 200, 'body' => $cached, 'retry_after' => null, 'cached' => true];
        }
        $url = $this->config->apiUrl() . '/api/code/?c=' . rawurlencode($code) . ($scan ? '&scan=1' : '');
        $response = $this->http->request('GET', $url, ['Accept: application/json', 'User-Agent: scutoons-qr']);
        if ($response === null || $response['status'] >= 500) return null;
        $body = json_decode($response['body'], true);
        if (!is_array($body)) return null;
        $retryAfter = null;
        foreach ($response['headers'] as [$name, $value]) {
            if (strtolower($name) === 'retry-after') $retryAfter = $value;
        }
        if ($response['status'] === 200) $this->cache->set('code:' . $code, $body, $this->config->int('CODE_CACHE_KEEP'));
        return ['status' => $response['status'], 'body' => $body, 'retry_after' => $retryAfter, 'cached' => false];
    }

    /** @return array<string, mixed>|null a cached /api/code/ answer at most $maxAge seconds old */
    public function cachedCode(string $code, int $maxAge): ?array
    {
        $value = $this->cache->get('code:' . $code, $maxAge);
        return is_array($value) ? $value : null;
    }

    /** @return array<string, array<string, mixed>>|null id => item, or null when the API is down and nothing is cached */
    public function items(): ?array
    {
        $ttl = $this->config->int('ITEMS_CACHE_TTL');
        $catalog = $this->cache->get('items', $ttl);
        if (!is_array($catalog)) {
            $response = $this->http->request('GET', $this->config->apiUrl() . '/api/items/', ['Accept: application/json', 'User-Agent: scutoons-qr']);
            $fresh = $response !== null && $response['status'] === 200 ? json_decode($response['body'], true) : null;
            if (is_array($fresh) && isset($fresh['items']) && is_array($fresh['items'])) {
                $catalog = $fresh;
                $this->cache->set('items', $catalog, $this->config->int('CODE_CACHE_KEEP'));
            } else {
                $catalog = $this->cache->get('items', $this->config->int('CODE_CACHE_KEEP'));
            }
        }
        if (!is_array($catalog) || !is_array($catalog['items'] ?? null)) return null;
        $items = [];
        foreach ($catalog['items'] as $item) {
            if (is_array($item) && isset($item['id'])) $items[(string) $item['id']] = $item;
        }
        return $items;
    }
}
