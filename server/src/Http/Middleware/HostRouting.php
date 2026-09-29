<?php
declare(strict_types=1);

namespace Scutoons\Http\Middleware;

use Psr\Http\Message\ResponseFactoryInterface;
use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Psr\Http\Server\MiddlewareInterface;
use Psr\Http\Server\RequestHandlerInterface as Handler;
use Scutoons\Domain\ApiError;
use Scutoons\Infrastructure\Config\Settings;

/**
 * One codebase, two subdomains (agents/PLATFORM.md "Domain architecture"): both point at public/, the host decides
 * what answers.
 *   QR_HOST  (qr.scutoons.com)  the public pages only: QR landing /?c=, verify (+ generate/get-signature.php and the
 *                               signature files it loads), /api/validate/, /api/keys/keys.php, static assets – and,
 *                               until the admin app replaces it, the legacy generate/ SPA.
 *                               TRANSITION: QR_HOST_SERVES_API (default on) keeps the full /api/* reachable here,
 *                               because released viewers call https://qr.scutoons.com/api/…
 *   API_HOST (api.scutoons.com) /api/* only (incl. /api/validate/ and /api/keys/keys.php) – no landing page.
 *   any other host (localhost, 127.0.0.1, a staging host) answers both roles (local development).
 * Anything a host does not serve → 404 (JSON `not_found` for /api paths, plain text for pages).
 */
final class HostRouting implements MiddlewareInterface
{
    /** Printed/public pages: landing, verify page data */
    public const PAGES = 'pages';
    /** Printed/public API URLs: /api/validate/, /api/keys/keys.php */
    public const PUBLIC_API = 'public_api';
    /** The rest of /api/* */
    public const API = 'api';
    /** The legacy generate/ admin SPA and its endpoints */
    public const LEGACY_ADMIN = 'legacy_admin';

    public function __construct(private readonly Settings $settings, private readonly ResponseFactoryInterface $responses)
    {
    }

    public function process(Request $request, Handler $handler): Response
    {
        $path = $request->getUri()->getPath();
        if (in_array(self::area($path), $this->areasFor($request->getUri()->getHost()), true)) return $handler->handle($request);

        if ($path === '/api' || str_starts_with($path, '/api/')) {
            return (new ErrorHandler($this->responses))->render(new ApiError(404, 'not_found', 'Unknown endpoint.'));
        }
        $response = $this->responses->createResponse(404)->withHeader('Content-Type', 'text/plain; charset=UTF-8');
        $response->getBody()->write('Not found');
        return $response;
    }

    /** Which part of the server a path belongs to */
    public static function area(string $path): string
    {
        if ($path === '/api/validate' || str_starts_with($path, '/api/validate/') || $path === '/api/keys/keys.php') return self::PUBLIC_API;
        if ($path === '/api' || str_starts_with($path, '/api/')) return self::API;
        if ($path === '/generate/get-signature.php' || str_starts_with($path, '/generate/files/signatures/')) return self::PAGES;
        if ($path === '/generate' || str_starts_with($path, '/generate/')) return self::LEGACY_ADMIN;
        return self::PAGES;
    }

    /** @return list<string> the areas a host serves */
    public function areasFor(string $host): array
    {
        $host = strtolower($host);
        if ($host !== '' && $host === strtolower($this->settings->get('API_HOST'))) return [self::API, self::PUBLIC_API];
        if ($host !== '' && $host === strtolower($this->settings->get('QR_HOST'))) {
            $areas = [self::PAGES, self::PUBLIC_API, self::LEGACY_ADMIN];
            if ($this->settings->bool('QR_HOST_SERVES_API')) $areas[] = self::API;
            return $areas;
        }
        return [self::PAGES, self::PUBLIC_API, self::API, self::LEGACY_ADMIN];
    }
}
