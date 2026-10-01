<?php
declare(strict_types=1);

namespace Scutoons\Http;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Infrastructure\Config\Settings;

/**
 * The single sign-on cookie (docs/API.md "Single sign-on"): the session token of `/auth/verify/` in an HttpOnly cookie,
 * so every SCUTOONS app (scutoons.com, creator., admin., shop., embassy.) shares one session.
 *
 *   scutoons_sid=<token>; Path=/api; Max-Age=<SESSION_TTL_DAYS>; HttpOnly; SameSite=Lax; Secure; Domain=scutoons.com
 *
 * - `Domain` only when the API host is SESSION_COOKIE_DOMAIN or a subdomain of it (production: api.scutoons.com and,
 *   during the transition, qr.scutoons.com/api through the qr app's proxy); any other host (localhost, 127.0.0.1)
 *   gets a host-only cookie – cookies ignore ports, so the local apps on :8000 … :13000 share it too.
 * - `Secure` always, except plain http on a loopback host (local development).
 * - `Path=/api`: the cookie never travels with page, CDN or asset requests of the other subdomains.
 * The token is the same as a Bearer token (one `sessions` row); the Bearer header wins when both are sent.
 */
final class SessionCookie
{
    public const NAME = 'scutoons_sid';
    public const PATH = '/api';

    public function __construct(private readonly Settings $settings)
    {
    }

    /** The well-formed session token of the request's cookie, or null */
    public static function token(Request $request): ?string
    {
        $value = $request->getCookieParams()[self::NAME] ?? null;
        return is_string($value) && preg_match('/^[A-Za-z0-9_-]{20,128}$/', $value) ? $value : null;
    }

    public function set(Request $request, Response $response, string $token): Response
    {
        return $response->withAddedHeader('Set-Cookie', $this->header($request, $token, 86400 * max(1, $this->settings->int('SESSION_TTL_DAYS'))));
    }

    /** Removes the cookie in the browser (logout) */
    public function clear(Request $request, Response $response): Response
    {
        return $response->withAddedHeader('Set-Cookie', $this->header($request, '', 0));
    }

    public function header(Request $request, string $value, int $maxAge): string
    {
        $host = strtolower($request->getUri()->getHost());
        $parts = [self::NAME . '=' . $value, 'Path=' . self::PATH, 'Max-Age=' . $maxAge];
        if ($maxAge === 0) $parts[] = 'Expires=Thu, 01 Jan 1970 00:00:00 GMT';
        $domain = $this->domainFor($host);
        if ($domain !== null) $parts[] = 'Domain=' . $domain;
        if (!($request->getUri()->getScheme() === 'http' && self::isLoopback($host))) $parts[] = 'Secure';
        $parts[] = 'HttpOnly';
        $parts[] = 'SameSite=Lax';
        return implode('; ', $parts);
    }

    /** SESSION_COOKIE_DOMAIN when the host is that domain or a subdomain of it, else null (host-only cookie) */
    public function domainFor(string $host): ?string
    {
        $domain = strtolower(ltrim(trim($this->settings->get('SESSION_COOKIE_DOMAIN')), '.'));
        if ($domain === '') return null;
        return $host === $domain || str_ends_with($host, '.' . $domain) ? $domain : null;
    }

    private static function isLoopback(string $host): bool
    {
        $host = trim($host, '[]');
        return $host === 'localhost' || $host === '127.0.0.1' || $host === '::1' || str_ends_with($host, '.localhost');
    }
}
