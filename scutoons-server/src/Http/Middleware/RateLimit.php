<?php
declare(strict_types=1);

namespace Scutoons\Http\Middleware;

use Closure;
use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Psr\Http\Server\MiddlewareInterface;
use Psr\Http\Server\RequestHandlerInterface as Handler;
use Scutoons\Domain\RateLimit\RateLimiter;

/**
 * Route guard: at most $max requests per bucket and window, every attempt counts (also failed ones – against code
 * enumeration). Over the limit → 429 rate_limited { retry_after }. Used for the public code lookups (/access/,
 * /code/: per client IP, ACCESS_MAX_PER_IP_HOUR). Limits that depend on the body or the user live in the services.
 */
final class RateLimit implements MiddlewareInterface
{
    /** @param Closure(Request): string $bucket */
    public function __construct(
        private readonly RateLimiter $limiter,
        private readonly Closure $bucket,
        private readonly int $max,
        private readonly int $windowSeconds,
        private readonly string $message,
    ) {
    }

    public function process(Request $request, Handler $handler): Response
    {
        $bucket = ($this->bucket)($request);
        $this->limiter->guard($bucket, $this->max, $this->windowSeconds, $this->message);
        $this->limiter->hit($bucket);
        $this->limiter->prune();
        return $handler->handle($request);
    }
}
