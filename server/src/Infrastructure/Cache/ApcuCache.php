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

    /** Only this prefix's entries (the pool may hold other apps' keys) */
    public function clear(): int
    {
        $removed = 0;
        foreach (new \APCUIterator('/^' . preg_quote($this->prefix, '/') . '/', APC_ITER_KEY) as $entry) {
            if (apcu_delete((string) $entry['key'])) $removed++;
        }
        return $removed;
    }
}
