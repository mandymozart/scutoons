<?php
declare(strict_types=1);

namespace Scutoons\Infrastructure\Clock;

final class SystemClock implements Clock
{
    public function time(): int
    {
        return time();
    }

    public function ms(): int
    {
        return (int) floor(microtime(true) * 1000);
    }

    public function utc(int $offsetSeconds = 0): string
    {
        return gmdate('Y-m-d H:i:s', time() + $offsetSeconds);
    }

    public function local(): string
    {
        return date('Y-m-d H:i:s');
    }
}
