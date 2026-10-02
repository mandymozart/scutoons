<?php
declare(strict_types=1);

namespace Scutoons\Domain\Mail;

use RuntimeException;

/** A mail could not be handed to the transport (already recorded as `failed` in the mail log, with the error) */
final class MailFailed extends RuntimeException
{
    public function __construct(public readonly string $template, public readonly int $logId, string $message, ?\Throwable $previous = null)
    {
        parent::__construct($message, 0, $previous);
    }
}
