<?php
declare(strict_types=1);

namespace Scutoons\Http\Middleware;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Psr\Http\Server\MiddlewareInterface;
use Psr\Http\Server\RequestHandlerInterface as Handler;

/**
 * API paths are matched without trailing slashes: `/api/me/` = `/api/me` (clients use the slash form, as documented in
 * agents/ITEMS.md). Runs before routing.
 */
final class NormalizePath implements MiddlewareInterface
{
    public function process(Request $request, Handler $handler): Response
    {
        $uri = $request->getUri();
        $path = $uri->getPath();
        if (($path === '/api/' || str_starts_with($path, '/api/')) && str_ends_with($path, '/')) {
            $request = $request->withUri($uri->withPath(rtrim($path, '/') ?: '/'));
        }
        return $handler->handle($request);
    }
}
