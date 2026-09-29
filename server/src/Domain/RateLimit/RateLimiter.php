<?php
declare(strict_types=1);

namespace Scutoons\Domain\RateLimit;

use Scutoons\Domain\ApiError;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Ids;

/**
 * Counts events per bucket in a time window (table rate_limits). A `ref` de-duplicates events (e.g. a story grant
 * with the same ref grants nothing the second time). No long-running process: rows are pruned now and then.
 */
final class RateLimiter
{
    public function __construct(private readonly Database $db, private readonly Clock $clock)
    {
    }

    public function count(string $bucket, int $windowSeconds): int
    {
        return (int) $this->db->value('SELECT COUNT(*) FROM rate_limits WHERE bucket = ? AND created_at > ?', [$bucket, $this->clock->utc(-$windowSeconds)]);
    }

    public function exists(string $bucket, string $ref): bool
    {
        return $this->db->value('SELECT 1 FROM rate_limits WHERE bucket = ? AND ref = ?', [$bucket, $ref]) !== null;
    }

    public function hit(string $bucket, ?string $ref = null): void
    {
        $this->db->run('INSERT INTO rate_limits (id, bucket, ref, created_at) VALUES (?, ?, ?, ?)',
            [Ids::uuid(), substr($bucket, 0, 191), $ref === null ? null : substr($ref, 0, 191), $this->clock->utc()]);
    }

    /** Throws 429 rate_limited (retry_after = the window) when `$max` events happened in the window */
    public function guard(string $bucket, int $max, int $windowSeconds, string $message): void
    {
        if ($this->count($bucket, $windowSeconds) >= $max) {
            throw new ApiError(429, 'rate_limited', $message, ['retry_after' => $windowSeconds]);
        }
    }

    /** Housekeeping (every ~50th call): drop events older than 30 days that carry no de-duplication ref */
    public function prune(): void
    {
        if (random_int(1, 50) !== 1) return;
        $this->db->run('DELETE FROM rate_limits WHERE ref IS NULL AND created_at < ?', [$this->clock->utc(-30 * 86400)]);
    }
}
