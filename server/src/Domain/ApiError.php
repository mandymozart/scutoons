<?php
declare(strict_types=1);

namespace Scutoons\Domain;

use Exception;

/**
 * An error of the API contract (agents/ITEMS.md): HTTP status + stable error code + message (+ extra fields).
 * Domain services throw it; the ErrorHandler middleware answers `{ status:"error", error, message, …extra }`.
 */
final class ApiError extends Exception
{
    /** @param array<string, mixed> $extra */
    public function __construct(
        public readonly int $status,
        public readonly string $errorCode,
        string $message = '',
        public readonly array $extra = [],
    ) {
        parent::__construct($message !== '' ? $message : $errorCode);
    }

    /** @return array<string, mixed> the JSON body */
    public function toArray(): array
    {
        return ['status' => 'error', 'error' => $this->errorCode, 'message' => $this->getMessage()] + $this->extra;
    }
}
