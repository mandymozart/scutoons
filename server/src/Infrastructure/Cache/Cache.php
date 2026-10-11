<?php
declare(strict_types=1);

namespace Scutoons\Infrastructure\Cache;

/**
 * A small string cache for values whose key already says what they are (content-addressed: the library keys contain
 * the version ids they were built from, agents/DATA.md §3), so nothing ever has to be invalidated – a stale key is
 * simply never asked for again. `ttl` only bounds how long unused entries occupy space. The exception is a data migration
 * that changes stored versions in place (BACKLOG 10.61): the developer's migration run calls clear() afterwards.
 */
interface Cache
{
    public function get(string $key): ?string;

    public function set(string $key, string $value, int $ttl): void;

    /** Removes every entry; returns how many (-1 = unknown) */
    public function clear(): int;
}
