<?php
declare(strict_types=1);

namespace Scutoons\Domain;

/**
 * Who is calling, as far as the domain needs it: client IP (rate limits, hashed for statistics), the browser's
 * random device id (X-Device-Id, hashed for statistics) and the Origin (where magic links point). Filled per request
 * by the DeviceId middleware; empty on the command line.
 */
final class RequestContext
{
    public const DEVICE_ID_PATTERN = '/^[A-Za-z0-9_-]{8,128}$/';

    public function __construct(
        private string $clientIp = '',
        private ?string $deviceId = null,
        private ?string $origin = null,
    ) {
    }

    public function set(string $clientIp, ?string $deviceId, ?string $origin): void
    {
        $this->clientIp = $clientIp;
        $this->deviceId = $deviceId !== null && preg_match(self::DEVICE_ID_PATTERN, $deviceId) ? $deviceId : null;
        $this->origin = $origin !== null && $origin !== '' ? rtrim($origin, '/') : null;
    }

    public function clientIp(): string
    {
        return $this->clientIp;
    }

    /** The valid X-Device-Id header, or null */
    public function deviceId(): ?string
    {
        return $this->deviceId;
    }

    public function origin(): ?string
    {
        return $this->origin;
    }
}
