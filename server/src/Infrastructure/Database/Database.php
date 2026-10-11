<?php
declare(strict_types=1);

namespace Scutoons\Infrastructure\Database;

use Closure;
use PDO;
use PDOStatement;
use Throwable;

/**
 * Thin PDO wrapper around MariaDB (production; locally the Docker container of `npm run dev:db`). Connects lazily
 * (endpoints that need no database never open one). Queries use positional parameters and PHP-side times.
 */
final class Database
{
    private ?PDO $pdo = null;

    /** @param Closure(): PDO $connect */
    public function __construct(private readonly Closure $connect)
    {
    }

    /**
     * @param string $sqlMode the session sql_mode set right after connecting ('' = keep the server's). The API passes
     *                        DB_SQL_MODE (strict by default): production's server default is NO_ENGINE_SUBSTITUTION,
     *                        which silently truncates / coerces bad values instead of failing.
     * @param string $collation the connection collation (`SET NAMES utf8mb4 COLLATE …`, '' = the server's): the tables'
     *                        own (utf8mb4_general_ci), so literals and user variables compare without #1267
     *                        "Illegal mix of collations" (BACKLOG 10.61 – phpMyAdmin's connection was utf8mb4_unicode_ci)
     */
    public static function mysql(string $host, string $name, string $user, string $password, string $port = '', string $sqlMode = '', string $collation = ''): self
    {
        $dsn = 'mysql:host=' . $host . ($port !== '' ? ';port=' . $port : '') . ';dbname=' . $name . ';charset=utf8mb4';
        if ($collation !== '' && preg_match('/^utf8mb4_\w+$/', $collation) !== 1) throw new \InvalidArgumentException("DB_COLLATION $collation is not a utf8mb4 collation");
        return new self(static function () use ($dsn, $user, $password, $sqlMode, $collation): PDO {
            $pdo = new PDO($dsn, $user, $password, [PDO::ATTR_EMULATE_PREPARES => false]);
            if ($collation !== '') $pdo->exec("SET NAMES utf8mb4 COLLATE $collation");
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
        return $this->pdo = $pdo;
    }

    /** Drops the connection (the next query connects again) */
    public function close(): void
    {
        $this->pdo = null;
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

    /**
     * Runs $work in a transaction and returns its result (row locks: SELECT … FOR UPDATE inside $work)
     * @template T
     * @param callable(): T $work
     * @return T
     */
    public function transaction(callable $work): mixed
    {
        $pdo = $this->pdo();
        // inside a transaction already (a batch of saves, EntityStore::saveMany): part of it – the outer one commits / rolls back
        if ($pdo->inTransaction()) return $work();
        $pdo->beginTransaction();
        try {
            $result = $work();
            $pdo->commit();
            return $result;
        } catch (Throwable $error) {
            $pdo->rollBack();
            throw $error;
        }
    }

    /** A LIKE pattern that matches $text anywhere (escape character "!": use `LIKE ? ESCAPE '!'`) */
    public static function likeContains(string $text): string
    {
        return '%' . strtr($text, ['!' => '!!', '%' => '!%', '_' => '!_']) . '%';
    }
}
