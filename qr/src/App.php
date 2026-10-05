<?php
declare(strict_types=1);

namespace Qr;

/**
 * qr.scutoons.com – every request that is not a static file in public/ (verify.html, assets/, version.json):
 *
 *   /api/validate[/…], /api/keys/keys.php     PRINTED: always proxied to the API, byte for byte
 *   /api[/…]                                  404 JSON (proxied only while PROXY_FULL_API is on – off since 10.5)
 *   /api/certificates[/…]                     the verify page's certificate check: always proxied to the API
 *   /generate/get-signature.php,
 *   /generate/files/signatures/SIGNATURE-*    (old verify page, signature downloads): proxied to the API
 *   /generate[/…]                             the legacy admin SPA lives on the API: 302 to API_URL/generate/…
 *   /version.json                             (dev only – deploys write the static file) version of this app
 *   everything else                           the QR landing page (Landing)
 */
final class App
{
    public function __construct(private readonly Config $config)
    {
    }

    public static function fromConfig(): self
    {
        return new self(Config::load());
    }

    public function run(): void
    {
        $method = strtoupper((string) ($_SERVER['REQUEST_METHOD'] ?? 'GET'));
        $uri = (string) ($_SERVER['REQUEST_URI'] ?? '/');
        $path = (string) parse_url($uri, PHP_URL_PATH);
        $query = (string) (parse_url($uri, PHP_URL_QUERY) ?? '');
        $response = $this->handle($method, $path === '' ? '/' : $path, $query);
        $response->send($method !== 'HEAD');
    }

    public function handle(string $method, string $path, string $query): Response
    {
        $http = new Http($this->config);
        if (self::isPublicApi($path) || self::isVerifyData($path)
            || (($path === '/api' || str_starts_with($path, '/api/')) && $this->config->bool('PROXY_FULL_API'))) {
            $answer = (new Proxy($this->config, $http))->forward($method, $path, $query, Proxy::incomingHeaders(), (string) file_get_contents('php://input'));
            return new Response($answer['status'], $answer['headers'], $answer['body']);
        }
        if ($path === '/api' || str_starts_with($path, '/api/')) {
            return Response::json(404, '{"status":"error","error":"not_found","message":"Unknown endpoint."}');
        }
        if ($path === '/generate' || str_starts_with($path, '/generate/')) {
            return Response::redirect($this->config->apiUrl() . $path . ($query !== '' ? '?' . $query : ''));
        }
        if ($path === '/version.json') {
            return Response::json(200, (string) json_encode(self::version($this->config->root()), JSON_UNESCAPED_SLASHES | JSON_PRETTY_PRINT));
        }
        parse_str($query, $params);
        $landing = new Landing($this->config, new Api($this->config, $http, Cache::fromConfig($this->config)), new View($this->config->root() . '/templates'));
        return $landing->handle($params, (string) ($_SERVER['HTTP_ACCEPT_LANGUAGE'] ?? ''));
    }

    /** /api/validate/ (+ index.php, no slash) and /api/keys/keys.php – printed, always proxied */
    public static function isPublicApi(string $path): bool
    {
        return $path === '/api/validate' || str_starts_with($path, '/api/validate/') || $path === '/api/keys/keys.php';
    }

    /** What the verify page loads from its own origin (+ the old verify page's signature sources) */
    public static function isVerifyData(string $path): bool
    {
        return $path === '/api/certificates' || str_starts_with($path, '/api/certificates/')
            || $path === '/generate/get-signature.php' || (bool) preg_match('#^/generate/files/signatures/SIGNATURE-[A-Za-z0-9_-]{1,32}\.json$#', $path);
    }

    /** @return array<string, mixed> version.json of a local checkout (deploys write public/version.json instead) */
    public static function version(string $root): array
    {
        $composer = json_decode((string) @file_get_contents($root . '/composer.json'), true);
        return ['app' => 'qr', 'version' => (string) ($composer['version'] ?? '0.0.0'), 'git_sha' => 'dev', 'built_at' => null, 'shared' => new \stdClass()];
    }
}
