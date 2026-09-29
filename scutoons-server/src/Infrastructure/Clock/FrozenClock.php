<?php
declare(strict_types=1);

namespace Scutoons\Infrastructure\Clock;

/** A clock that only moves when told to (unit tests) */
final class FrozenClock implements Clock
{
    public function __construct(private int $time)
    {
    }

    public function advance(int $seconds): void
    {
        $this->time += $seconds;
    }

    public function time(): int
    {
        return $this->time;
    }

    public function ms(): int
    {
        return $this->time * 1000;
    }

    public function utc(int $offsetSeconds = 0): string
    {
        return gmdate('Y-m-d H:i:s', $this->time + $offsetSeconds);
    }

    public function local(): string
    {
        return date('Y-m-d H:i:s', $this->time);
    }
}
