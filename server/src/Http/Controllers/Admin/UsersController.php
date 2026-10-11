<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers\Admin;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\Admin\ListQuery;
use Scutoons\Domain\Admin\UserAdmin;
use Scutoons\Domain\Content\Reviews;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;
use Scutoons\Infrastructure\Config\Settings;

/**
 * /api/admin/users and /api/admin/roles – role admin (docs/API.md "Admin API"):
 *   GET    /users                        list (page, page_size, sort, q = part of the email, role, opt_in)
 *   GET    /users/{id}                   account detail (roles, owned codes, sessions)
 *   PUT    /users/{id}/roles/{role}      grant  → { changed, user }
 *   DELETE /users/{id}/roles/{role}      revoke → { changed, user }
 *   PUT    /users/{id}/characters/{c}    unlock an avatar-body character { looks?, source?, ref? } → { changed, characters }
 *   DELETE /users/{id}/characters/{c}    revoke it (with its looks) → { changed, characters }
 *   DELETE /users/{id}/characters/{c}/looks/{look}   revoke one A-pose look → { changed, characters }
 *   GET    /roles                        every role
 *   GET    /review-rules                 every review rule (Domain\Content\Reviews) → { items }
 *   PUT    /users/{id}/review-rules/{toon}/{rule}   add (toon id or *, rule reviewer | needs_review) → { changed, review_rules }
 *   DELETE /users/{id}/review-rules/{toon}/{rule}   remove → { changed, review_rules }
 */
final class UsersController
{
    public function __construct(private readonly UserAdmin $users, private readonly Roles $roles, private readonly Settings $settings, private readonly Reviews $reviews)
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
        return Json::response($response, 200, ['status' => 'success'] + $this->users->setRole($args['id'], $args['role'], true, self::by($request), self::actor($request)));
    }

    /** @param array{id: string, role: string} $args */
    public function revoke(Request $request, Response $response, array $args): Response
    {
        return Json::response($response, 200, ['status' => 'success'] + $this->users->setRole($args['id'], $args['role'], false, self::by($request), self::actor($request)));
    }

    /** @param array{id: string, character: string} $args */
    public function unlockCharacter(Request $request, Response $response, array $args): Response
    {
        $body = JsonBody::of($request);
        return Json::response($response, 200, ['status' => 'success'] + $this->users->unlockCharacter($args['id'], $args['character'], $body, self::by($request)));
    }

    /** @param array{id: string, character: string} $args */
    public function lockCharacter(Request $request, Response $response, array $args): Response
    {
        return Json::response($response, 200, ['status' => 'success'] + $this->users->lockCharacter($args['id'], $args['character']));
    }

    /** @param array{id: string, character: string, look: string} $args */
    public function lockLook(Request $request, Response $response, array $args): Response
    {
        return Json::response($response, 200, ['status' => 'success'] + $this->users->lockCharacter($args['id'], $args['character'], $args['look']));
    }

    public function roles(Request $request, Response $response): Response
    {
        return Json::response($response, 200, ['status' => 'success', 'items' => $this->roles->all()]);
    }

    public function reviewRules(Request $request, Response $response): Response
    {
        return Json::response($response, 200, ['status' => 'success', 'items' => $this->reviews->rules()]);
    }

    /** @param array{id: string, toon: string, rule: string} $args */
    public function addReviewRule(Request $request, Response $response, array $args): Response
    {
        return $this->reviewRule($request, $response, $args, true);
    }

    /** @param array{id: string, toon: string, rule: string} $args */
    public function removeReviewRule(Request $request, Response $response, array $args): Response
    {
        return $this->reviewRule($request, $response, $args, false);
    }

    /** @param array{id: string, toon: string, rule: string} $args */
    private function reviewRule(Request $request, Response $response, array $args, bool $on): Response
    {
        $this->users->detail($args['id']); // 404 for unknown accounts
        $changed = $this->reviews->setRule($args['id'], $args['toon'], $args['rule'], $on, self::by($request));
        return Json::response($response, 200, ['status' => 'success', 'changed' => $changed, 'review_rules' => $this->reviews->rules($args['id'])]);
    }

    /** @return array<string, mixed>|null the signed-in admin */
    private static function actor(Request $request): ?array
    {
        $admin = $request->getAttribute('user');
        return is_array($admin) ? $admin : null;
    }

    /** user_roles.granted_by: "admin:<email>" of the signed-in admin */
    private static function by(Request $request): string
    {
        $admin = $request->getAttribute('user');
        return 'admin:' . (is_array($admin) ? (string) ($admin['email'] ?? '') : '');
    }
}
