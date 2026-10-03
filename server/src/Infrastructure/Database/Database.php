<?php
declare(strict_types=1);

namespace Scutoons\Infrastructure\Database;

use Closure;
use PDO;
use PDOStatement;
use Throwable;

/**
 * Thin PDO wrapper: MySQL/MariaDB in production, SQLite locally. Connects lazily (endpoints that need no database
 * never open one). Queries use positional parameters and PHP-side times so the same SQL runs on both drivers.
 */
final class Database
{
    private ?PDO $pdo = null;

    /** @param Closure(): PDO $connect */
    public function __construct(private readonly Closure $connect)
    {
    }

    public static function sqlite(string $path): self
    {
        return new self(static fn(): PDO => new PDO('sqlite:' . $path));
    }

    /**
     * @param string $sqlMode the session sql_mode set right after connecting ('' = keep the server's). The API passes
     *                        DB_SQL_MODE (strict by default): production's server default is NO_ENGINE_SUBSTITUTION,
     *                        which silently truncates / coerces bad values instead of failing like SQLite tests expect.
     */
    public static function mysql(string $host, string $name, string $user, string $password, string $port = '', string $sqlMode = ''): self
    {
        $dsn = 'mysql:host=' . $host . ($port !== '' ? ';port=' . $port : '') . ';dbname=' . $name . ';charset=utf8mb4';
        return new self(static function () use ($dsn, $user, $password, $sqlMode): PDO {
            $pdo = new PDO($dsn, $user, $password, [PDO::ATTR_EMULATE_PREPARES => false]);
            if ($sqlMode !== '') $pdo->prepare('SET SESSION sql_mode = ?')->execute([$sqlMode]);
            return $pdo;
        });
    }

    public function pdo(): PDO
    {
        if ($this->pdo !== null) return $this->pdo;
        $pdo = ($this->connect)();
        $pdo->setAttribute(PDO::ATTR_ERRMODE, PDO::ERRMODE_EXCEPTION);
        $pdo->setAttribute(PDO::ATTR_DEFAULT_FETCH_MODE, PDO::FETCH_ASSOC);
        if ($pdo->getAttribute(PDO::ATTR_DRIVER_NAME) === 'sqlite') {
            $pdo->exec('PRAGMA foreign_keys = ON');
            $pdo->exec('PRAGMA busy_timeout = 5000');
        }
        return $this->pdo = $pdo;
    }

    /** Drops the connection (the next query connects again) – e.g. before deleting an SQLite file */
    public function close(): void
    {
        $this->pdo = null;
    }

    /** "sqlite" or "mysql" */
    public function driver(): string
    {
        return $this->pdo()->getAttribute(PDO::ATTR_DRIVER_NAME) === 'sqlite' ? 'sqlite' : 'mysql';
    }

    public function isSqlite(): bool
    {
        return $this->driver() === 'sqlite';
    }

    /** @param list<mixed> $params */
    public function run(string $sql, array $params = []): PDOStatement
    {
        $statement = $this->pdo()->prepare($sql);
        $statement->execute($params);
        return $statement;
    }

    /**
     * @param list<mixed> $params
     * @return array<string, mixed>|null
     */
    public function one(string $sql, array $params = []): ?array
    {
        /** @var array<string, mixed>|false $row */
        $row = $this->run($sql, $params)->fetch();
        return $row === false ? null : $row;
    }

    /**
     * @param list<mixed> $params
     * @return list<array<string, mixed>>
     */
    public function all(string $sql, array $params = []): array
    {
        /** @var list<array<string, mixed>> */
        return $this->run($sql, $params)->fetchAll();
    }

    /** @param list<mixed> $params */
    public function value(string $sql, array $params = []): mixed
    {
        $value = $this->run($sql, $params)->fetchColumn();
        return $value === false ? null : $value;
    }

    public function exec(string $sql): void
    {
        $this->pdo()->exec($sql);
    }

    public function lastInsertId(): int
    {
        return (int) $this->pdo()->lastInsertId();
    }

    /** ` FOR UPDATE` on MySQL (row lock inside a transaction), nothing on SQLite (the write lock covers it) */
    public function forUpdate(): string
    {
        return $this->isSqlite() ? '' : ' FOR UPDATE';
    }

    /**
     * Runs $work in a transaction and returns its result. SQLite takes the write lock up front (BEGIN IMMEDIATE),
     * so concurrent claims serialize; MySQL relies on SELECT … FOR UPDATE inside $work.
     * @template T
     * @param callable(): T $work
     * @return T
     */
    public function transaction(callable $work): mixed
    {
        $pdo = $this->pdo();
        $sqlite = $this->isSqlite();
        $sqlite ? $pdo->exec('BEGIN IMMEDIATE') : $pdo->beginTransaction();
        try {
            $result = $work();
            $sqlite ? $pdo->exec('COMMIT') : $pdo->commit();
            return $result;
        } catch (Throwable $error) {
            $sqlite ? $pdo->exec('ROLLBACK') : $pdo->rollBack();
            throw $error;
        }
    }

    /** A LIKE pattern that matches $text anywhere (escape character "!": use `LIKE ? ESCAPE '!'`) */
    public static function likeContains(string $text): string
    {
        return '%' . strtr($text, ['!' => '!!', '%' => '!%', '_' => '!_']) . '%';
    }
}
