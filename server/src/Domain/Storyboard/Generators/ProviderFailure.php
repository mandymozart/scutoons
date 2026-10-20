<?php
declare(strict_types=1);

namespace Scutoons\Domain\Storyboard\Generators;

use RuntimeException;

/**
 * A provider call that failed: `status` = the provider's HTTP status (0 = no answer – a timeout or a broken connection,
 * so a start call may have been billed all the same).
 */
final class ProviderFailure extends RuntimeException
{
    public function __construct(string $message, public readonly int $status)
    {
        parent::__construct($message);
    }

    /** the provider answered (a refused start is not billed) */
    public function answered(): bool
    {
        return $this->status > 0;
    }
}
