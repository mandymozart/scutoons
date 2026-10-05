<?php
declare(strict_types=1);

namespace Qr;

/**
 * Passes a request through to the API unchanged and sends its answer back unchanged: status, body (byte for byte) and
 * the headers (content type, caching, CORS, ETag, Location, …) – PRINTED URLS /api/validate/ and /api/keys/keys.php,
 * the verify page's generate/get-signature.php + signature files, and (PROXY_FULL_API) the whole /api/*.
 * The client's IP and X-Device-Id go along in the trusted headers (Http).
 */
final class Proxy
{
    /**
     * Request headers never forwarded: hop-by-hop, the host, compression (the body must stay identical), cookies – except
     * the API's single sign-on cookie (SESSION_COOKIE), which goes along alone: with PROXY_FULL_API on (the old
     * viewer's transition, off since CUTOVER 10.5) apps called the API through this proxy (qr.scutoons.com/api), and
     * the cookie (Domain=scutoons.com, Path=/api) is theirs.
     */
    public const SESSION_COOKIE = 'scutoons_sid';
    private const DROP_REQUEST = ['host', 'connection', 'keep-alive', 'proxy-authorization', 'proxy-connection', 'te',
        'trailer', 'transfer-encoding', 'upgrade', 'content-length', 'accept-encoding', 'cookie', 'expect'];
    /** Response headers never passed back: hop-by-hop, lengths PHP computes itself, the API's server banner */
    private const DROP_RESPONSE = ['connection', 'keep-alive', 'proxy-authenticate', 'te', 'trailer', 'transfer-encoding',
        'upgrade', 'content-length', 'content-encoding', 'date', 'server', 'x-powered-by', 'host'];

    public function __construct(private readonly Config $config, private readonly Http $http)
    {
    }

    /**
     * @param array<string, string> $requestHeaders
     * @return array{status: int, headers: list<array{0: string, 1: string}>, body: string}
     */
    public function forward(string $method, string $path, string $query, array $requestHeaders, string $body): array
    {
        $headers = [];
        foreach ($requestHeaders as $name => $value) {
            $lower = strtolower((string) $name);
            if (in_array($lower, self::DROP_REQUEST, true) || str_starts_with($lower, 'x-scutoons-')) continue;
            $headers[] = $name . ': ' . $value;
        }
        $session = self::sessionCookie($requestHeaders);
        if ($session !== null) $headers[] = 'Cookie: ' . self::SESSION_COOKIE . '=' . $session;
        $url = $this->config->apiUrl() . $path . ($query !== '' ? '?' . $query : '');
        $response = $this->http->request($method, $url, $headers, $body, max(1, $this->config->int('PROXY_TIMEOUT')));
        if ($response === null) {
            return ['status' => 502, 'headers' => [['Content-Type', 'application/json; charset=utf-8'], ['Cache-Control', 'no-store'],
                ['Retry-After', '60'], ['Access-Control-Allow-Origin', '*']],
                'body' => '{"status":"error","error":"api_unavailable","message":"The server is not reachable – try again in a minute."}'];
        }
        $response['headers'] = array_values(array_filter($response['headers'],
            static fn(array $h): bool => !in_array(strtolower($h[0]), self::DROP_RESPONSE, true)));
        return $response;
    }

    /**
     * The well-formed value of the single sign-on cookie in the request's Cookie header, or null
     * @param array<string, string> $requestHeaders
     */
    public static function sessionCookie(array $requestHeaders): ?string
    {
        foreach ($requestHeaders as $name => $value) {
            if (strtolower((string) $name) !== 'cookie') continue;
            foreach (explode(';', $value) as $pair) {
                [$key, $cookie] = array_pad(explode('=', trim($pair), 2), 2, '');
                if ($key === self::SESSION_COOKIE && preg_match('/^[A-Za-z0-9_-]{20,128}$/', $cookie)) return $cookie;
            }
        }
        return null;
    }

    /** @return array<string, string> the incoming request headers (Apache, FastCGI and php -S) */
    public static function incomingHeaders(): array
    {
        $headers = [];
        foreach ($_SERVER as $key => $value) {
            if (!is_string($value)) continue;
            if (str_starts_with($key, 'HTTP_')) {
                $headers[str_replace(' ', '-', ucwords(strtolower(str_replace('_', ' ', substr($key, 5)))))] = $value;
            } elseif (in_array($key, ['CONTENT_TYPE', 'CONTENT_LENGTH'], true) && $value !== '') {
                $headers[str_replace(' ', '-', ucwords(strtolower(str_replace('_', ' ', $key))))] = $value;
            }
        }
        // FastCGI hosts drop Authorization unless .htaccess copies it into the environment
        if (!isset($headers['Authorization'])) {
            $auth = $_SERVER['REDIRECT_HTTP_AUTHORIZATION'] ?? (function_exists('getallheaders') ? (getallheaders()['Authorization'] ?? null) : null);
            if (is_string($auth) && $auth !== '') $headers['Authorization'] = $auth;
        }
        return $headers;
    }
}
