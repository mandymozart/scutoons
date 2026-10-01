<?php
declare(strict_types=1);

namespace Scutoons\Http\Middleware;

use Psr\Http\Message\ResponseFactoryInterface;
use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Psr\Http\Server\MiddlewareInterface;
use Psr\Http\Server\RequestHandlerInterface as Handler;
use Scutoons\Http\AllowedOrigins;
use Scutoons\Infrastructure\Config\Settings;

/**
 * CORS for the API (/api/*): browsers on an ALLOWED_ORIGINS origin may call it with a Bearer token or with the single
 * sign-on cookie (`credentials: 'include'` → `Access-Control-Allow-Credentials: true`, only ever with the exact origin,
 * never with `*`). A preflight (OPTIONS) ends here with 204. Error responses carry the headers too (this runs outside
 * the ErrorHandler).
 *
 * Kept exactly as the plain-PHP server answered (contract): /api/toons/ and /api/keys/keys.php send no CORS headers;
 * /api/validate/ is open to every origin (`*`, legacy viewer builds); /api/items/ and /api/editions/ add `*` for
 * other origins in their controller. /api/admin/* and /api/embassy/* also allow DELETE.
 */
final class Cors implements MiddlewareInterface
{
    private const METHODS = 'GET, POST, PUT, OPTIONS';
    private const ADMIN_METHODS = 'GET, POST, PUT, DELETE, OPTIONS';
    private const HEADERS = 'Content-Type, Authorization, X-Device-Id';

    public function __construct(private readonly Settings $settings, private readonly ResponseFactoryInterface $responses)
    {
    }

    public function process(Request $request, Handler $handler): Response
    {
        $path = $request->getUri()->getPath();
        if (!self::applies($path)) return $handler->handle($request);

        $response = $request->getMethod() === 'OPTIONS' ? $this->responses->createResponse(204) : $handler->handle($request);

        if (self::isValidate($path)) {
            $response = $response->withHeader('Access-Control-Allow-Origin', '*')
                ->withHeader('Access-Control-Allow-Methods', 'GET, POST, OPTIONS')
                ->withHeader('Access-Control-Allow-Headers', 'Content-Type, Authorization');
        }
        $origin = rtrim($request->getHeaderLine('Origin'), '/');
        if ($origin !== '' && $this->isAllowed($origin)) {
            $response = $response->withHeader('Access-Control-Allow-Origin', $origin)
                ->withHeader('Vary', 'Origin')
                ->withHeader('Access-Control-Allow-Methods', str_starts_with($path, '/api/admin') || str_starts_with($path, '/api/embassy') ? self::ADMIN_METHODS : self::METHODS)
                ->withHeader('Access-Control-Allow-Headers', self::HEADERS)
                ->withHeader('Access-Control-Allow-Credentials', 'true')
                ->withHeader('Access-Control-Max-Age', '86400');
        }
        return $response;
    }

    public function isAllowed(string $origin): bool
    {
        return AllowedOrigins::allows($this->settings, $origin);
    }

    private static function applies(string $path): bool
    {
        if ($path !== '/api' && !str_starts_with($path, '/api/')) return false;
        return !str_starts_with($path, '/api/toons') && !str_starts_with($path, '/api/keys/');
    }

    private static function isValidate(string $path): bool
    {
        return $path === '/api/validate' || str_starts_with($path, '/api/validate/');
    }
}
