<?php
declare(strict_types=1);

/**
 * Boot file of the contract suites (api-smoke.php, content-smoke.php, claim-race.php). The suites were written
 * against the plain-PHP server, which had static classes (Db::, Codes::, Roles::, …). Their assertions stay
 * unchanged: this file provides the same static calls as thin facades over the refactored services (one container
 * per process, the configured database – QR_DB_SQLITE_PATH for a copy). Test-only; never loaded by the server.
 */

use Scutoons\Domain\Access\AccessTokens;
use Scutoons\Domain\Access\Permissions as PermissionsService;
use Scutoons\Domain\Accounts\Users as UsersService;
use Scutoons\Domain\Auth\LoginService;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Domain\Catalog\Catalog as CatalogService;
use Scutoons\Domain\Codes\CodeService;
use Scutoons\Domain\Content\EntityStore;
use Scutoons\Domain\Roles\Roles as RolesService;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Kernel;
use Scutoons\Support\Ids;

require_once __DIR__ . '/../../vendor/autoload.php';

class_alias(Scutoons\Domain\ApiError::class, 'ApiError');

final class TestContainer
{
    private static ?DI\Container $container = null;

    /**
     * @template T of object
     * @param class-string<T> $class
     * @return T
     */
    public static function get(string $class): object
    {
        self::$container ??= Kernel::container();
        return self::$container->get($class);
    }
}

// config.local.php (loaded by Settings) defines GENERATE_PASSWORD and DB_SQLITE_PATH locally
TestContainer::get(Settings::class);
if (!defined('GENERATE_PASSWORD')) define('GENERATE_PASSWORD', TestContainer::get(Settings::class)->get('GENERATE_PASSWORD'));

/** Server-timezone `Y-m-d H:i:s` (the `qr` table's time format) */
function qrNow(): string
{
    return TestContainer::get(Clock::class)->local();
}

final class Config
{
    public static function get(string $key): string
    {
        $settings = TestContainer::get(Settings::class);
        return str_ends_with($key, '_PATH') ? $settings->path($key) : $settings->get($key);
    }
}

final class Db
{
    /** @param list<mixed> $params */
    public static function run(string $sql, array $params = []): PDOStatement
    {
        return TestContainer::get(Database::class)->run($sql, $params);
    }

    /** @param list<mixed> $params @return array<string, mixed>|null */
    public static function one(string $sql, array $params = []): ?array
    {
        return TestContainer::get(Database::class)->one($sql, $params);
    }

    /** @param list<mixed> $params @return list<array<string, mixed>> */
    public static function all(string $sql, array $params = []): array
    {
        return TestContainer::get(Database::class)->all($sql, $params);
    }

    /** @param list<mixed> $params */
    public static function value(string $sql, array $params = []): mixed
    {
        return TestContainer::get(Database::class)->value($sql, $params);
    }

    public static function transaction(callable $work): mixed
    {
        return TestContainer::get(Database::class)->transaction($work);
    }

    public static function now(int $offsetSeconds = 0): string
    {
        return TestContainer::get(Clock::class)->utc($offsetSeconds);
    }

    public static function uuid(): string
    {
        return Ids::uuid();
    }

    public static function uuid7(): string
    {
        return Ids::uuid7();
    }
}

final class Codes
{
    /** @return array<string, mixed>|null */
    public static function find(string $code): ?array
    {
        return TestContainer::get(CodeService::class)->find($code);
    }

    /** @param array<string, mixed> $user */
    public static function claim(array $user, string $code, ?string $itemId, ?string $variant): bool
    {
        return TestContainer::get(CodeService::class)->claim($user, $code, $itemId, $variant);
    }
}

final class Users
{
    /** @return array<string, mixed>|null */
    public static function byId(string $id): ?array
    {
        return TestContainer::get(UsersService::class)->byId($id);
    }

    /** @return array<string, mixed> */
    public static function create(string $email, string $locale, ?string $signupCode = null, bool $marketingOptIn = false): array
    {
        return TestContainer::get(UsersService::class)->create($email, $locale, $signupCode, $marketingOptIn);
    }
}

final class Auth
{
    /**
     * @param array<string, mixed> $request
     * @param array<string, mixed> $body
     * @return array<string, mixed>
     */
    public static function confirm(array $request, array $body): array
    {
        return TestContainer::get(LoginService::class)->confirm($request, $body);
    }

    public static function createSession(string $userId): string
    {
        return TestContainer::get(Sessions::class)->create($userId);
    }
}

final class Roles
{
    public const MEMBER = RolesService::MEMBER;
    public const ADMIN = RolesService::ADMIN;
    public const CREATOR = RolesService::CREATOR;

    /** @param array<string, mixed>|string $user */
    public static function has(array|string $user, string $role): bool
    {
        return TestContainer::get(RolesService::class)->has($user, $role);
    }

    public static function grant(string $userId, string $role, ?string $grantedBy = null): bool
    {
        return TestContainer::get(RolesService::class)->grant($userId, $role, $grantedBy);
    }

    /**
     * The user of `$_SERVER['HTTP_AUTHORIZATION']` (Bearer) with the role, else ApiError 401/403 – as the
     * RequireRole middleware decides it
     * @return array<string, mixed>
     */
    public static function requireRole(string $role): array
    {
        $token = Sessions::tokenFromHeader((string) ($_SERVER['HTTP_AUTHORIZATION'] ?? ''));
        $user = $token !== null ? TestContainer::get(Sessions::class)->user($token) : null;
        return TestContainer::get(RolesService::class)->require($user, $role);
    }
}

final class Permissions
{
    public const TOKEN_MAX_SECONDS = AccessTokens::MAX_SECONDS;

    public static function useFile(?string $path): void
    {
        TestContainer::get(PermissionsService::class)->useFile($path);
    }

    public static function claimable(string $type): bool
    {
        return TestContainer::get(PermissionsService::class)->claimable($type);
    }

    /**
     * @param list<array<string, mixed>> $rows
     * @return array<string, mixed>
     */
    public static function access(array $rows, string $userId): array
    {
        return TestContainer::get(PermissionsService::class)->access($rows, $userId);
    }

    /** @param list<string> $permissions */
    public static function issueToken(array $permissions, ?int $codeExpires, string $subject): string
    {
        return TestContainer::get(AccessTokens::class)->issue($permissions, $codeExpires, $subject);
    }

    /** @return array<string, mixed>|null */
    public static function verifyToken(string $token): ?array
    {
        return TestContainer::get(AccessTokens::class)->verify($token);
    }
}

final class Catalog
{
    /** @return list<string> */
    public static function slots(): array
    {
        return TestContainer::get(CatalogService::class)->slots();
    }
}

final class Content
{
    public static function maxBytes(): int
    {
        return TestContainer::get(EntityStore::class)->maxBytes();
    }
}
