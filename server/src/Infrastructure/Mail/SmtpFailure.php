<?php
declare(strict_types=1);

namespace Scutoons\Infrastructure\Mail;

use RuntimeException;

/**
 * An SMTP conversation that went wrong: at which step (connect | tls | auth | mail | rcpt | data) and the server's reply
 * code (0 = no reply, e.g. the connection failed). The newsletter sender (Domain\Newsletter\NewsletterSender) tells a
 * bad address (5xx at rcpt – never try it again) from a problem of the account or the server (stop the batch).
 */
final class SmtpFailure extends RuntimeException
{
    public function __construct(string $message, public readonly string $stage, public readonly int $reply = 0)
    {
        parent::__construct($message, $reply);
    }

    /** The server refused this recipient for good (550 no such mailbox, 553 bad address, …) */
    public function permanentForRecipient(): bool
    {
        return $this->stage === 'rcpt' && $this->reply >= 500 && $this->reply < 600;
    }

    /** The account or the server is the problem (login, connection, TLS): every further mail of the batch would fail too */
    public function aboutTheServer(): bool
    {
        return in_array($this->stage, ['connect', 'tls', 'auth'], true);
    }
}
