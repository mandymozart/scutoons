<?php
declare(strict_types=1);

namespace Scutoons\Infrastructure\Cache;

/** Shared memory of the PHP-FPM pool (APCu), when the host has it */
final class ApcuCache implements Cache
{
    public function __construct(private readonly string $prefix = 'scutoons:')
    {
    }

    public static function available(): bool
    {
        return function_exists('apcu_enabled') && apcu_enabled();
    }

    public function get(string $key): ?string
    {
        $value = apcu_fetch($this->prefix . $key, $found);
        return $found && is_string($value) ? $value : null;
    }

    public function set(string $key, string $value, int $ttl): void
    {
        apcu_store($this->prefix . $key, $value, $ttl);
    }
}
