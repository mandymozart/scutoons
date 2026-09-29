<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers\Admin;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\Access\Permissions;
use Scutoons\Domain\Admin\Dashboard;
use Scutoons\Domain\Admin\EventsAdmin;
use Scutoons\Domain\Admin\ListQuery;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Catalog\Catalog;
use Scutoons\Domain\Catalog\Editions;
use Scutoons\Domain\Catalog\Toons;
use Scutoons\Http\Json;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Signing\KeyStore;

/**
 * /api/admin – role admin (docs/API.md "Admin API"):
 *   GET /dashboard   KPIs (?days=30, ?edition_id=)
 *   GET /events      activity log (page, page_size, sort, q, code, event, edition_id, toon, from, to)
 *   GET /catalog     items, editions, toons and the (server-only) permissions map, read-only
 *   GET /keys        the ML-DSA-65 keypair for client-side signing of certificates
 */
final class SystemController
{
    public function __construct(
        private readonly Dashboard $dashboard,
        private readonly EventsAdmin $events,
        private readonly Catalog $catalog,
        private readonly Editions $editions,
        private readonly Toons $toons,
        private readonly Permissions $permissions,
        private readonly KeyStore $keys,
        private readonly Settings $settings,
    ) {
    }

    public function dashboard(Request $request, Response $response): Response
    {
        $query = $request->getQueryParams();
        $edition = is_string($query['edition_id'] ?? null) && $query['edition_id'] !== '' ? $query['edition_id'] : null;
        return Json::response($response, 200, $this->dashboard->build((int) ($query['days'] ?? 30), $edition));
    }

    public function events(Request $request, Response $response): Response
    {
        $query = ListQuery::from($request->getQueryParams(), EventsAdmin::SORTABLE, '-created_at', EventsAdmin::FILTERS, 100, $this->settings->int('ADMIN_MAX_PAGE_SIZE'));
        return Json::response($response, 200, ['status' => 'success'] + $this->events->list($query));
    }

    public function catalog(Request $request, Response $response): Response
    {
        return Json::response($response, 200, [
            'status' => 'success',
            'catalog' => $this->catalog->data(),
            'editions' => $this->editions->all(),
            'toons' => $this->toons->all(),
            'permissions' => $this->permissions->map(),
        ]);
    }

    public function keys(Request $request, Response $response): Response
    {
        $keys = $this->keys->keypair();
        if ($keys === null) throw new ApiError(404, 'keys_missing', 'No keypair stored yet (KEYPAIR_PATH). See docs/KEY-MANAGEMENT.md.');
        if (!isset($keys['privateKey'], $keys['publicKey'])) throw new ApiError(500, 'keys_invalid', 'The stored keypair is invalid.');
        return Json::response($response, 200, [
            'status' => 'success',
            'algorithm' => $keys['algorithm'] ?? 'ML-DSA-65',
            'publicKey' => $keys['publicKey'],
            'privateKey' => $keys['privateKey'],
            'generated' => $keys['generated'] ?? null,
        ]);
    }
}
