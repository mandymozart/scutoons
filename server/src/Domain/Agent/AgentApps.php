<?php
declare(strict_types=1);

namespace Scutoons\Domain\Agent;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Embassy\Embassy;
use Scutoons\Domain\Roles\Roles;

/**
 * The apps with an Ask panel (docs/API.md "Agent chats" / "Agent usage"): every chat and every logged AI call belongs to
 * one, and who may use it follows that app's own access – `creator`: a content editor (Roles::requireContentEditor),
 * `embassy`: the Embassy access (liaison / ambassador / admin, Embassy::access), `admin`: the admin role.
 */
final class AgentApps
{
    public const CREATOR = 'creator';
    public const EMBASSY = 'embassy';
    public const ADMIN = 'admin';
    public const APPS = [self::CREATOR, self::EMBASSY, self::ADMIN];

    public function __construct(private readonly Roles $roles, private readonly Embassy $embassy)
    {
    }

    /** A client's `app` value: null / '' = creator · 422 invalid_app */
    public static function app(mixed $value): string
    {
        if ($value === null || $value === '') return self::CREATOR;
        if (!is_string($value) || !in_array($value, self::APPS, true)) {
            throw new ApiError(422, 'invalid_app', 'app must be one of ' . implode(', ', self::APPS) . '.', ['allowed' => self::APPS]);
        }
        return $value;
    }

    /**
     * The signed-in user when they may use the app's Ask panel: 401 unauthorized / 403 forbidden { role } otherwise
     * @param array<string, mixed>|null $user
     * @return array<string, mixed>
     */
    public function requireUser(string $app, ?array $user): array
    {
        if ($user === null) throw new ApiError(401, 'unauthorized', 'Not signed in.');
        if ($app === self::EMBASSY) {
            $this->embassy->access($user);
            return $user;
        }
        return $app === self::ADMIN ? $this->roles->require($user, Roles::ADMIN) : $this->roles->requireContentEditor($user);
    }
}
