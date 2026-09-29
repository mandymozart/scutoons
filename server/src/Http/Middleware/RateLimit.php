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
 * /code/: per client IP; QR scans through the qr app in their own bucket – config/routes.php). The limit may be a
 * closure (read per request, e.g. a runtime setting); 0 = no limit (nothing is counted). Limits that depend on the
 * body or the user live in the services.
 */
final class RateLimit implements MiddlewareInterface
{
    /**
     * @param Closure(Request): string $bucket
     * @param int|Closure(Request): int $max
     */
    public function __construct(
        private readonly RateLimiter $limiter,
        private readonly Closure $bucket,
        private readonly int|Closure $max,
        private readonly int $windowSeconds,
        private readonly string $message,
    ) {
    }

    public function process(Request $request, Handler $handler): Response
    {
        $max = $this->max instanceof Closure ? ($this->max)($request) : $this->max;
        if ($max <= 0) return $handler->handle($request);
        $bucket = ($this->bucket)($request);
        $this->limiter->guard($bucket, $max, $this->windowSeconds, $this->message);
        $this->limiter->hit($bucket);
        $this->limiter->prune();
        return $handler->handle($request);
    }
}
