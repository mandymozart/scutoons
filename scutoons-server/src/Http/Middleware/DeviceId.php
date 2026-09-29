<?php
declare(strict_types=1);

namespace Scutoons\Http\Middleware;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Psr\Http\Server\MiddlewareInterface;
use Psr\Http\Server\RequestHandlerInterface as Handler;
use Scutoons\Domain\RequestContext;

/**
 * Fills the RequestContext of the domain: the browser's random `X-Device-Id` (valid form only; statistics store its
 * HMAC), the client IP (REMOTE_ADDR – rate limits, daily-salted hash in statistics) and the Origin (magic links).
 */
final class DeviceId implements MiddlewareInterface
{
    public function __construct(private readonly RequestContext $context)
    {
    }

    public function process(Request $request, Handler $handler): Response
    {
        $server = $request->getServerParams();
        $this->context->set(
            (string) ($server['REMOTE_ADDR'] ?? ''),
            $request->getHeaderLine('X-Device-Id') ?: null,
            $request->getHeaderLine('Origin') ?: null,
        );
        return $handler->handle($request);
    }
}
