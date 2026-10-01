<?php
declare(strict_types=1);

namespace Scutoons\Http;

use Scutoons\Infrastructure\Config\Settings;

/** ALLOWED_ORIGINS: the browser origins of the SCUTOONS apps (CORS with credentials, the cookie's CSRF check) */
final class AllowedOrigins
{
    public static function allows(Settings $settings, string $origin): bool
    {
        $origin = rtrim($origin, '/');
        if ($origin === '') return false;
        foreach ($settings->allowedOrigins() as $allowed) { // ALLOWED_ORIGINS + the local dev ports on a dev server
            if (strcasecmp(rtrim($allowed, '/'), $origin) === 0) return true;
        }
        return false;
    }
}
