<?php
declare(strict_types=1);

namespace Scutoons\Tests\Infrastructure;

use RuntimeException;
use Scutoons\Infrastructure\Mail\Mailbox;

/** An in-memory IMAP mailbox: one folder's mails by UID, the appended copies, and a switch to fail */
final class FakeMailbox implements Mailbox
{
    /** @var array<int, string> UID → raw mail */
    public array $mails = [];
    /** @var list<array{folder: string, message: string}> */
    public array $appended = [];
    /** @var list<array{after: int, since: string}> */
    public array $searches = [];
    public int $uidValidity = 1;
    public ?string $failOn = null;
    public bool $open = false;
    public int $closed = 0;

    public function open(): void
    {
        if ($this->failOn === 'open') throw new RuntimeException('IMAP connect to ssl://imap.test:993 failed: refused (111)');
        $this->open = true;
    }

    public function select(string $folder): int
    {
        $this->assertOpen();
        return $this->uidValidity;
    }

    public function search(int $afterUid, string $since): array
    {
        $this->assertOpen();
        $this->searches[] = ['after' => $afterUid, 'since' => $since];
        $uids = array_values(array_filter(array_keys($this->mails), fn(int $uid) => $uid > $afterUid));
        sort($uids);
        return $uids;
    }

    public function fetch(int $uid, int $maxBytes): string
    {
        $this->assertOpen();
        return substr($this->mails[$uid] ?? '', 0, $maxBytes);
    }

    public function append(string $folder, string $message): void
    {
        $this->assertOpen();
        if ($this->failOn === 'append') throw new RuntimeException('IMAP: APPEND failed: NO [TRYCREATE] no such folder');
        $this->appended[] = ['folder' => $folder, 'message' => $message];
    }

    public function close(): void
    {
        $this->open = false;
        $this->closed++;
    }

    private function assertOpen(): void
    {
        if (!$this->open) throw new RuntimeException('IMAP not connected');
    }
}
