<?php
declare(strict_types=1);

namespace Scutoons\Infrastructure\Mail;

use RuntimeException;

/**
 * An IMAP mailbox as the Embassy's inbox sync needs it (Imap; tests use a fake). Every method throws RuntimeException
 * when the server refuses or the connection fails. One connection: open() → select() → search() / fetch() → close();
 * append() works on an open connection without a select.
 */
interface Mailbox
{
    /** Connects and signs in (EMBASSY_IMAP_*) */
    public function open(): void;

    /** Selects a folder read-only; returns its UIDVALIDITY */
    public function select(string $folder): int;

    /**
     * UIDs of the selected folder: all above $afterUid, or – with $afterUid = 0 – the mails since $since (YYYY-MM-DD)
     * @return list<int> ascending
     */
    public function search(int $afterUid, string $since): array;

    /** The raw message (at most $maxBytes; not marked as read) */
    public function fetch(int $uid, int $maxBytes): string;

    /** Stores a raw message in a folder, marked as read (a copy of a sent mail) */
    public function append(string $folder, string $message): void;

    /** Signs out and disconnects (never throws) */
    public function close(): void;
}
