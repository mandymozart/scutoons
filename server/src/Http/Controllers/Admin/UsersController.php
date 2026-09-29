<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers\Admin;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\Admin\ListQuery;
use Scutoons\Domain\Admin\UserAdmin;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Http\Json;
use Scutoons\Infrastructure\Config\Settings;

/**
 * /api/admin/users and /api/admin/roles – role admin (docs/API.md "Admin API"):
 *   GET    /users                        list (page, page_size, sort, q = part of the email, role, opt_in)
 *   GET    /users/{id}                   account detail (roles, owned codes, sessions)
 *   PUT    /users/{id}/roles/{role}      grant  → { changed, user }
 *   DELETE /users/{id}/roles/{role}      revoke → { changed, user }
 *   GET    /roles                        every role
 */
final class UsersController
{
    public function __construct(private readonly UserAdmin $users, private readonly Roles $roles, private readonly Settings $settings)
    {
    }

    public function list(Request $request, Response $response): Response
    {
        $query = ListQuery::from($request->getQueryParams(), UserAdmin::SORTABLE, '-created_at', UserAdmin::FILTERS, 50, $this->settings->int('ADMIN_MAX_PAGE_SIZE'));
        return Json::response($response, 200, ['status' => 'success'] + $this->users->list($query));
    }

    /** @param array{id: string} $args */
    public function get(Request $request, Response $response, array $args): Response
    {
        return Json::response($response, 200, ['status' => 'success', 'user' => $this->users->detail($args['id'])]);
    }

    /** @param array{id: string, role: string} $args */
    public function grant(Request $request, Response $response, array $args): Response
    {
        return Json::response($response, 200, ['status' => 'success'] + $this->users->setRole($args['id'], $args['role'], true, self::by($request)));
    }

    /** @param array{id: string, role: string} $args */
    public function revoke(Request $request, Response $response, array $args): Response
    {
        return Json::response($response, 200, ['status' => 'success'] + $this->users->setRole($args['id'], $args['role'], false, self::by($request)));
    }

    public function roles(Request $request, Response $response): Response
    {
        return Json::response($response, 200, ['status' => 'success', 'items' => $this->roles->all()]);
    }

    /** user_roles.granted_by: "admin:<email>" of the signed-in admin */
    private static function by(Request $request): string
    {
        $admin = $request->getAttribute('user');
        return 'admin:' . (is_array($admin) ? (string) ($admin['email'] ?? '') : '');
    }
}
