<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Catalog\FeaturedProducts;
use Scutoons\Domain\Catalog\Products;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\Cors;

/**
 * The shop's public product data, identical for everyone (like /api/items/):
 *   GET /api/products/           the shop catalog (version 2; tables catalog_products + meta `products`)
 *   GET /api/products/featured/  the landing page's editions [{ product, variant? }]: the list chosen in the admin app
 *                                (table catalog_featured)
 * Both in the shape of the old files (objects, key order kept). Any origin may read both (`*` for origins outside ALLOWED_ORIGINS),
 * the catalog cached 5 minutes, the featured list 1 minute (an admin change shows up soon). Other methods → 405
 * method_not_allowed (like /api/items/).
 */
final class ProductsController
{
    public function __construct(private readonly Products $products, private readonly FeaturedProducts $featured, private readonly Cors $cors)
    {
    }

    public function catalog(Request $request, Response $response): Response
    {
        return $this->public($request, $response, fn() => $this->products->catalogJson());
    }

    public function featured(Request $request, Response $response): Response
    {
        return $this->public($request, $response, fn() => $this->featured->json(), 60);
    }

    /** @param callable(): string $json */
    private function public(Request $request, Response $response, callable $json, int $maxAge = 300): Response
    {
        if ($request->getMethod() !== 'GET') {
            throw new ApiError(405, 'method_not_allowed', 'Use GET.');
        }
        $origin = rtrim($request->getHeaderLine('Origin'), '/');
        return Json::raw($response, 200, $json(), 'application/json; charset=utf-8')
            ->withHeader('Access-Control-Allow-Origin', $origin !== '' && $this->cors->isAllowed($origin) ? $origin : '*')
            ->withHeader('Cache-Control', "public, max-age=$maxAge");
    }
}
