<?php
declare(strict_types=1);

namespace Qr;

/**
 * Settings of the qr app. Defaults below; per installation `config.php` (production, written by the deploy) or
 * `config.local.php` (local, gitignored) return an array that overrides them; an environment variable QRAPP_<KEY>
 * wins over both (the contract tests start qr with it).
 */
final class Config
{
    public const DEFAULTS = [
        // The API, server-side (no /api suffix). Local: http://localhost:9000
        'API_URL' => 'https://api.scutoons.com',
        // Shared secret with the API (its QR_PROXY_SECRET): lets the API trust the client IP / device id we forward
        'QR_PROXY_SECRET' => '',
        // TRANSITION: proxy the whole /api/* to the API (released viewers call https://qr.scutoons.com/api/…).
        // /api/validate/ and /api/keys/keys.php are always proxied (printed URLs). Switch off (0) when the stats show
        // no more API traffic on qr.
        'PROXY_FULL_API' => '1',
        // The viewer (old magic links /?login= go there; account link fallback)
        'WWW_URL' => 'https://scutoons.com',
        // Item icons: <CDN_URL><icon path>
        'CDN_URL' => 'https://cdn.scutoons.com/',
        // Timeouts in seconds: page lookups (a scan must answer fast) and proxied API calls
        'API_CONNECT_TIMEOUT' => '2',
        'API_TIMEOUT' => '5',
        'PROXY_TIMEOUT' => '30',
        // Code lookups are cached briefly (skip=1 redirects use them), and kept longer only for the fallback page
        // ("Continue" when the API is down). The item catalog is cached a bit longer.
        'CODE_CACHE_TTL' => '60',
        'CODE_CACHE_KEEP' => '86400',
        'ITEMS_CACHE_TTL' => '300',
        // File cache when APCu is not available (relative to the qr folder; never served)
        'CACHE_DIR' => 'cache',
    ];

    /** @param array<string, string> $values */
    private function __construct(private readonly string $root, private readonly array $values)
    {
    }

    public static function load(?string $root = null): self
    {
        $root ??= dirname(__DIR__);
        $values = self::DEFAULTS;
        foreach (['config.php', 'config.local.php'] as $file) {
            $path = $root . '/' . $file;
            if (!is_file($path)) continue;
            $data = require $path;
            if (is_array($data)) {
                foreach ($data as $key => $value) {
                    if (array_key_exists((string) $key, $values)) $values[(string) $key] = (string) $value;
                }
            }
            break; // config.php (production) wins; config.local.php only when there is none
        }
        foreach (array_keys($values) as $key) {
            $env = getenv('QRAPP_' . $key);
            if ($env !== false) $values[$key] = $env;
        }
        return new self($root, $values);
    }

    /** @param array<string, string> $values */
    public static function fixed(string $root, array $values): self
    {
        return new self($root, array_merge(self::DEFAULTS, $values));
    }

    public function root(): string
    {
        return $this->root;
    }

    public function get(string $key): string
    {
        return $this->values[$key] ?? throw new \InvalidArgumentException("Unknown setting $key");
    }

    public function int(string $key): int
    {
        return (int) $this->get($key);
    }

    public function bool(string $key): bool
    {
        return in_array(strtolower($this->get($key)), ['1', 'true', 'on', 'yes'], true);
    }

    public function apiUrl(): string
    {
        return rtrim($this->get('API_URL'), '/');
    }

    public function path(string $key): string
    {
        $value = $this->get($key);
        return preg_match('#^([A-Za-z]:)?[/\\\\]#', $value) ? $value : $this->root . '/' . $value;
    }
}
