<?php
declare(strict_types=1);

namespace Scutoons\Infrastructure\Cache;

/** CACHE_DRIVER=none: every read is a miss */
final class NullCache implements Cache
{
    public function get(string $key): ?string
    {
        return null;
    }

    public function set(string $key, string $value, int $ttl): void
    {
    }
}
