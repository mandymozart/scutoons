<?php
declare(strict_types=1);

namespace Scutoons\Infrastructure\Clock;

/**
 * The current time. Every service asks this instead of time(), so tests can freeze it (FrozenClock).
 *
 * Two storage formats exist (kept from the plain-PHP server):
 * - account tables (users, sessions, inventory, code_events, …): UTC `Y-m-d H:i:s` → utc()
 * - the `qr` table (created_at, claimed_at, revoked_at): the server's timezone → local()
 */
interface Clock
{
    /** Unix time in seconds */
    public function time(): int;

    /** Unix time in milliseconds */
    public function ms(): int;

    /** UTC `Y-m-d H:i:s`, optionally shifted by $offsetSeconds (as stored in the account tables) */
    public function utc(int $offsetSeconds = 0): string;

    /** Server-timezone `Y-m-d H:i:s` (as stored in the `qr` table) */
    public function local(): string;
}
