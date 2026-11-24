<?php
declare(strict_types=1);

namespace Scutoons\Domain\Thumbnails;

use PDOException;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Database\Database;

/**
 * The `thumbnails` table (migration 2026-11-26-thumbnails) – rows only, shared by Thumbnails (uploads, lookups) and
 * AssetJobs (the stills the server makes). Without the migration reads return nothing and writes are skipped.
 */
final class ThumbnailRows
{
    public function __construct(private readonly Database $db, private readonly Clock $clock)
    {
    }

    public function available(): bool
    {
        try {
            $this->db->value('SELECT 1 FROM thumbnails LIMIT 1');
            return true;
        } catch (PDOException $error) {
            if (self::missing($error)) return false;
            throw $error;
        }
    }

    /**
     * Rows of these keys: shared ones (owner '') and the member's own (owner = $userId, only workspace keys ask for it)
     * @param list<string> $shared
     * @param list<string> $own
     * @return array<string, array<string, mixed>> key → row
     */
    public function of(array $shared, array $own, ?string $userId): array
    {
        $where = [];
        $params = [];
        if ($shared !== []) {
            $where[] = "(owner = '' AND thumb_key IN (" . implode(',', array_fill(0, count($shared), '?')) . '))';
            array_push($params, ...$shared);
        }
        if ($own !== [] && $userId !== null && $userId !== '') {
            $where[] = '(owner = ? AND thumb_key IN (' . implode(',', array_fill(0, count($own), '?')) . '))';
            array_push($params, $userId, ...$own);
        }
        if ($where === []) return [];
        try {
            $rows = $this->db->all('SELECT * FROM thumbnails WHERE ' . implode(' OR ', $where), $params);
        } catch (PDOException $error) {
            if (self::missing($error)) return [];
            throw $error;
        }
        $out = [];
        foreach ($rows as $r) $out[(string) $r['thumb_key']] = $r;
        return $out;
    }

    /** @return array<string, mixed>|null */
    public function one(string $owner, string $key): ?array
    {
        try {
            return $this->db->one('SELECT * FROM thumbnails WHERE owner = ? AND thumb_key = ?', [$owner, $key]);
        } catch (PDOException $error) {
            if (self::missing($error)) return null;
            throw $error;
        }
    }

    /** Insert or replace a still; false: the table is not there yet */
    public function record(string $owner, string $key, string $path, string $stamp, string $origin, int $size, ?string $by): bool
    {
        try {
            $this->db->run(
                'INSERT INTO thumbnails (owner, thumb_key, path, stamp, stale, origin, size, taken_at, taken_by) VALUES (?, ?, ?, ?, 0, ?, ?, ?, ?)
                 ON DUPLICATE KEY UPDATE path = VALUES(path), stamp = VALUES(stamp), stale = 0, origin = VALUES(origin), size = VALUES(size), taken_at = VALUES(taken_at), taken_by = VALUES(taken_by)',
                [$owner, $key, $path, mb_substr($stamp, 0, 100), $origin, $size, $this->clock->utc(), $by]
            );
            return true;
        } catch (PDOException $error) {
            if (self::missing($error)) return false;
            throw $error;
        }
    }

    /** A shared still waits to be made again (its thing changed) */
    public function markStale(string $key): void
    {
        try {
            $this->db->run("UPDATE thumbnails SET stale = 1 WHERE owner = '' AND thumb_key = ?", [$key]);
        } catch (PDOException $error) {
            if (!self::missing($error)) throw $error;
        }
    }

    /** MariaDB 42S02 (table) / 42S22 (column): the migration has not run yet */
    public static function missing(PDOException $error): bool
    {
        return in_array($error->getCode(), ['42S02', '42S22'], true) || preg_match('/Base table or view not found|Unknown column/i', $error->getMessage()) === 1;
    }
}
