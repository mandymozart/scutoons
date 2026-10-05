<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers\Admin;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Catalog\FeaturedProducts;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;

/**
 * /api/admin/featured – role admin (docs/API.md "Admin API"): the editions on the landing page (Domain\Catalog\FeaturedProducts).
 *   GET  /featured   → { status, items: [{ product, variant? }], updated_at, updated_by, max }
 *   PUT  /featured   { items: [{ product, variant? }] } → { status, changed, …same }
 */
final class FeaturedController
{
    public function __construct(private readonly FeaturedProducts $featured)
    {
    }

    public function get(Request $request, Response $response): Response
    {
        return Json::response($response, 200, ['status' => 'success'] + $this->featured->state());
    }

    public function put(Request $request, Response $response): Response
    {
        $body = JsonBody::of($request);
        if (!array_key_exists('items', $body)) {
            throw new ApiError(400, 'invalid_featured', 'Send { items: [{ product, variant? }] }.');
        }
        $admin = $request->getAttribute('user');
        $by = 'admin:' . (is_array($admin) ? (string) ($admin['email'] ?? '') : '');
        $changed = $this->featured->update($body['items'], $by);
        return Json::response($response, 200, ['status' => 'success', 'changed' => $changed] + $this->featured->state());
    }
}
