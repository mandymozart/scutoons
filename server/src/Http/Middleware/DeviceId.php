<?php
declare(strict_types=1);

namespace Scutoons\Http\Middleware;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Psr\Http\Server\MiddlewareInterface;
use Psr\Http\Server\RequestHandlerInterface as Handler;
use Scutoons\Domain\RequestContext;
use Scutoons\Infrastructure\Config\Settings;

/**
 * Fills the RequestContext of the domain: the browser's random `X-Device-Id` (valid form only; statistics store its
 * HMAC), the client IP (REMOTE_ADDR – rate limits, daily-salted hash in statistics) and the Origin (magic links).
 *
 * Trusted proxy (the qr app, qr.scutoons.com, which calls the API server-side and proxies /api/*): when QR_PROXY_SECRET
 * is set and the request carries it in `X-Scutoons-Proxy-Secret`, the client IP comes from `X-Scutoons-Client-Ip` and
 * the device id from `X-Scutoons-Device-Id` (falling back to `X-Device-Id`). Without the secret those headers are
 * ignored, so nobody can fake an IP to dodge the rate limits.
 */
final class DeviceId implements MiddlewareInterface
{
    public const SECRET_HEADER = 'X-Scutoons-Proxy-Secret';
    public const CLIENT_IP_HEADER = 'X-Scutoons-Client-Ip';
    public const DEVICE_ID_HEADER = 'X-Scutoons-Device-Id';

    public function __construct(private readonly RequestContext $context, private readonly Settings $settings)
    {
    }

    public function process(Request $request, Handler $handler): Response
    {
        $server = $request->getServerParams();
        $ip = (string) ($server['REMOTE_ADDR'] ?? '');
        $deviceId = $request->getHeaderLine('X-Device-Id') ?: null;
        $trusted = $this->trusted($request);
        if ($trusted) {
            $forwardedIp = trim($request->getHeaderLine(self::CLIENT_IP_HEADER));
            if (filter_var($forwardedIp, FILTER_VALIDATE_IP) !== false) $ip = $forwardedIp;
            $deviceId = $request->getHeaderLine(self::DEVICE_ID_HEADER) ?: $deviceId;
        }
        $this->context->set($ip, $deviceId, $request->getHeaderLine('Origin') ?: null, $trusted);
        return $handler->handle($request);
    }

    private function trusted(Request $request): bool
    {
        $secret = $this->settings->get('QR_PROXY_SECRET');
        $sent = $request->getHeaderLine(self::SECRET_HEADER);
        return $secret !== '' && $sent !== '' && hash_equals($secret, $sent);
    }
}
