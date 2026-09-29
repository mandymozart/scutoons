<?php
declare(strict_types=1);

/**
 * Thin PDO helper on top of getDbConnection() (db.php: MySQL in production, SQLite from config.local.php locally).
 * Queries use positional parameters and PHP-side times so they run on both.
 */
final class Db
{
    private static ?PDO $pdo = null;

    public static function pdo(): PDO
    {
        if (self::$pdo) return self::$pdo;
        $pdo = getDbConnection();
        $pdo->setAttribute(PDO::ATTR_ERRMODE, PDO::ERRMODE_EXCEPTION);
        $pdo->setAttribute(PDO::ATTR_DEFAULT_FETCH_MODE, PDO::FETCH_ASSOC);
        if ($pdo->getAttribute(PDO::ATTR_DRIVER_NAME) === 'sqlite') {
            $pdo->exec('PRAGMA foreign_keys = ON');
            $pdo->exec('PRAGMA busy_timeout = 5000');
        }
        return self::$pdo = $pdo;
    }

    public static function isSqlite(): bool
    {
        return self::pdo()->getAttribute(PDO::ATTR_DRIVER_NAME) === 'sqlite';
    }

    /** @param list<mixed> $params */
    public static function run(string $sql, array $params = []): PDOStatement
    {
        $statement = self::pdo()->prepare($sql);
        $statement->execute($params);
        return $statement;
    }

    /**
     * @param list<mixed> $params
     * @return array<string, mixed>|null
     */
    public static function one(string $sql, array $params = []): ?array
    {
        $row = self::run($sql, $params)->fetch();
        return $row === false ? null : $row;
    }

    /**
     * @param list<mixed> $params
     * @return list<array<string, mixed>>
     */
    public static function all(string $sql, array $params = []): array
    {
        return self::run($sql, $params)->fetchAll();
    }

    /** @param list<mixed> $params */
    public static function value(string $sql, array $params = []): mixed
    {
        $value = self::run($sql, $params)->fetchColumn();
        return $value === false ? null : $value;
    }

    /** ` FOR UPDATE` on MySQL (row lock inside a transaction), nothing on SQLite (the write lock covers it) */
    public static function forUpdate(): string
    {
        return self::isSqlite() ? '' : ' FOR UPDATE';
    }

    private static int $depth = 0;

    /** Runs $work in a transaction; nested calls join the outer transaction (all or nothing) */
    public static function transaction(callable $work): mixed
    {
        if (self::$depth > 0) {
            self::$depth++;
            try {
                return $work();
            } finally {
                self::$depth--;
            }
        }
        $pdo = self::pdo();
        self::$depth = 1;
        try {
            return self::outerTransaction($pdo, $work);
        } finally {
            self::$depth = 0;
        }
    }

    private static function outerTransaction(PDO $pdo, callable $work): mixed
    {
        if (self::isSqlite()) {
            // Take the write lock up front so concurrent claims serialize
            $pdo->exec('BEGIN IMMEDIATE');
        } else {
            $pdo->beginTransaction();
        }
        try {
            $result = $work();
            self::isSqlite() ? $pdo->exec('COMMIT') : $pdo->commit();
            return $result;
        } catch (Throwable $error) {
            self::isSqlite() ? $pdo->exec('ROLLBACK') : $pdo->rollBack();
            throw $error;
        }
    }

    /** UTC time as stored in the DATETIME columns of the account tables */
    public static function now(int $offsetSeconds = 0): string
    {
        return gmdate('Y-m-d H:i:s', time() + $offsetSeconds);
    }

    /** DATETIME (UTC) → ISO 8601 */
    public static function iso(?string $value): ?string
    {
        if ($value === null || $value === '') return null;
        $time = strtotime($value . ' UTC');
        return $time === false ? null : gmdate('c', $time);
    }

    public static function uuid(): string
    {
        $bytes = random_bytes(16);
        $bytes[6] = chr((ord($bytes[6]) & 0x0f) | 0x40);
        $bytes[8] = chr((ord($bytes[8]) & 0x3f) | 0x80);
        return vsprintf('%s%s-%s-%s-%s-%s%s%s', str_split(bin2hex($bytes), 4));
    }
}
