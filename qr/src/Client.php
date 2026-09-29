<?php
declare(strict_types=1);

namespace Qr;

/** The visitor as the API should see it: IP (REMOTE_ADDR) and the browser's X-Device-Id (valid form only) */
final class Client
{
    public function __construct(public readonly string $ip, public readonly ?string $deviceId)
    {
    }

    public static function fromGlobals(): self
    {
        $device = $_SERVER['HTTP_X_DEVICE_ID'] ?? null;
        return new self(
            (string) ($_SERVER['REMOTE_ADDR'] ?? ''),
            is_string($device) && preg_match('/^[A-Za-z0-9_-]{8,128}$/', $device) ? $device : null,
        );
    }
}
