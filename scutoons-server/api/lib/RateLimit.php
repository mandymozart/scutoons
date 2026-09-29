<?php
declare(strict_types=1);

/** Counting events per bucket in a time window (table rate_limits); `ref` de-duplicates events */
final class RateLimit
{
    public static function count(string $bucket, int $windowSeconds): int
    {
        return (int) Db::value('SELECT COUNT(*) FROM rate_limits WHERE bucket = ? AND created_at > ?', [$bucket, Db::now(-$windowSeconds)]);
    }

    public static function exists(string $bucket, string $ref): bool
    {
        return Db::value('SELECT 1 FROM rate_limits WHERE bucket = ? AND ref = ?', [$bucket, $ref]) !== null;
    }

    public static function hit(string $bucket, ?string $ref = null): void
    {
        Db::run('INSERT INTO rate_limits (id, bucket, ref, created_at) VALUES (?, ?, ?, ?)',
            [Db::uuid(), substr($bucket, 0, 191), $ref === null ? null : substr($ref, 0, 191), Db::now()]);
    }

    /** Throws 429 when `$max` events happened in the window */
    public static function guard(string $bucket, int $max, int $windowSeconds, string $message): void
    {
        if (self::count($bucket, $windowSeconds) >= $max) {
            throw new ApiError(429, 'rate_limited', $message, ['retry_after' => $windowSeconds]);
        }
    }

    /** Housekeeping: drop events older than 30 days that carry no de-duplication ref */
    public static function prune(): void
    {
        if (random_int(1, 50) !== 1) return;
        Db::run('DELETE FROM rate_limits WHERE ref IS NULL AND created_at < ?', [Db::now(-30 * 86400)]);
    }
}
