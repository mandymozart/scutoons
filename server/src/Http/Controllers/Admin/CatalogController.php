<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers\Admin;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Catalog\CatalogAdmin;
use Scutoons\Domain\Catalog\CatalogStore;
use Scutoons\Domain\Content\Reviews;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;

/**
 * /api/admin/catalog/… – role admin: the catalog tables (Domain\Catalog\CatalogAdmin, agents/CATALOG.md, docs/API.md
 * "Admin API → Catalog"). <kind> = items | recipes | albums | products | editions | featured | permissions | code-types |
 * toons | meta.
 *   GET    /catalog/<kind>               → { status, kind, items: [{ id, position, data, updated_at, updated_by }] }
 *   GET    /catalog/<kind>/<id>          → { status, entry }
 *   PUT    /catalog/<kind>/<id>          { data } → { status, created, changed, entry, history_id } (422 invalid_catalog + problems)
 *   DELETE /catalog/<kind>/<id>          → { status, deleted, history_id } (409 in_use + uses)
 *   POST   /catalog/<kind>/reorder       { ids: [...] } → { status, changed, ids, history_id }
 *   GET    /catalog/history              ?kind=&id=&limit= → { status, items: [{ id, kind, entry_id, action, before, after, … }] }
 *   POST   /catalog/history/<n>/undo     { force? } → { status, undone, history_id, entry? } (409 changed_since)
 * Creating a toon (PUT /catalog/toons/<id> { data: { id, name, redirectUrl } }) also creates its content index entry
 * `toon:<id>` (a draft) when the creator has none yet.
 * A physical change of an item / recipe / album / product / edition (put, delete, undo) needs the review rule `physical`
 * for its toons – admins too (Reviews::guardPhysical, agents/CATALOG-IN-CREATOR.md): 403 review_required.
 */
final class CatalogController
{
    public function __construct(private readonly CatalogAdmin $admin, private readonly Reviews $reviews)
    {
    }

    /** @param array{kind: string} $args */
    public function list(Request $request, Response $response, array $args): Response
    {
        return Json::response($response, 200, ['status' => 'success'] + $this->admin->list($args['kind']));
    }

    /** @param array{kind: string, id: string} $args */
    public function get(Request $request, Response $response, array $args): Response
    {
        return Json::response($response, 200, ['status' => 'success', 'entry' => $this->admin->entry($args['kind'], $args['id'])]);
    }

    /** @param array{kind: string, id: string} $args */
    public function put(Request $request, Response $response, array $args): Response
    {
        CatalogStore::kind($args['kind']);
        $body = JsonBody::object($request);
        if ($body === null || !property_exists($body, 'data')) throw new ApiError(400, 'missing_data', 'Send { data: <the entry> }.');
        [$by, $userId] = self::who($request);
        $data = json_decode((string) json_encode($body->data), true);
        $this->reviews->guardPhysical((string) $userId, $args['kind'], $args['id'], is_array($data) ? $data : null);
        $result = $this->admin->put($args['kind'], $args['id'], $body->data, $by, $userId, isset($body->note) && is_string($body->note) ? $body->note : null);
        return Json::response($response, $result['created'] ? 201 : 200, ['status' => 'success'] + $result);
    }

    /** @param array{kind: string, id: string} $args */
    public function delete(Request $request, Response $response, array $args): Response
    {
        [$by, $userId] = self::who($request);
        $this->reviews->guardPhysical((string) $userId, $args['kind'], $args['id'], null);
        return Json::response($response, 200, ['status' => 'success'] + $this->admin->delete($args['kind'], $args['id'], $by));
    }

    /** @param array{kind: string} $args */
    public function reorder(Request $request, Response $response, array $args): Response
    {
        $body = JsonBody::of($request);
        [$by] = self::who($request);
        return Json::response($response, 200, ['status' => 'success'] + $this->admin->reorder($args['kind'], $body['ids'] ?? null, $by));
    }

    public function history(Request $request, Response $response): Response
    {
        $q = $request->getQueryParams();
        $kind = is_string($q['kind'] ?? null) ? $q['kind'] : null;
        $id = is_string($q['id'] ?? null) ? $q['id'] : null;
        return Json::response($response, 200, ['status' => 'success', 'items' => $this->admin->history($kind, $id, (int) ($q['limit'] ?? 50))]);
    }

    /** @param array{id: string} $args */
    public function undo(Request $request, Response $response, array $args): Response
    {
        $body = JsonBody::of($request);
        [$by, $userId] = self::who($request);
        $change = $this->admin->historyRow((int) $args['id']);
        if ($change !== null && $change['entry_id'] !== '') {
            $before = $change['before_json'] !== null ? json_decode((string) $change['before_json'], true) : null;
            $this->reviews->guardPhysical((string) $userId, (string) $change['kind'], (string) $change['entry_id'], is_array($before) ? $before : null);
        }
        return Json::response($response, 200, ['status' => 'success'] + $this->admin->undo((int) $args['id'], $by, ($body['force'] ?? false) === true, $userId));
    }

    /** @return array{0: string, 1: ?string} [admin:<email>, user id] */
    private static function who(Request $request): array
    {
        $admin = $request->getAttribute('user');
        return ['admin:' . (is_array($admin) ? (string) ($admin['email'] ?? '') : ''), is_array($admin) && isset($admin['id']) ? (string) $admin['id'] : null];
    }
}
