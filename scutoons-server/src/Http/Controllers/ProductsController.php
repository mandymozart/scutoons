<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Catalog\Products;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\Cors;

/**
 * The shop's public product data, identical for everyone (like /api/items/):
 *   GET /api/products/           config/products.json (shop catalog, version 2)
 *   GET /api/products/featured/  config/featured-products.json (homepage slides: [{ product, variant? }])
 * The body is the stored file byte-for-byte. Any origin may read it (`*` for origins outside ALLOWED_ORIGINS),
 * cached 5 minutes. Other methods → 405 method_not_allowed (like /api/items/).
 */
final class ProductsController
{
    public function __construct(private readonly Products $products, private readonly Cors $cors)
    {
    }

    public function catalog(Request $request, Response $response): Response
    {
        return $this->public($request, $response, fn() => $this->products->catalogJson());
    }

    public function featured(Request $request, Response $response): Response
    {
        return $this->public($request, $response, fn() => $this->products->featuredJson());
    }

    /** @param callable(): string $json */
    private function public(Request $request, Response $response, callable $json): Response
    {
        if ($request->getMethod() !== 'GET') {
            throw new ApiError(405, 'method_not_allowed', 'Use GET.');
        }
        $origin = rtrim($request->getHeaderLine('Origin'), '/');
        return Json::raw($response, 200, $json(), 'application/json; charset=utf-8')
            ->withHeader('Access-Control-Allow-Origin', $origin !== '' && $this->cors->isAllowed($origin) ? $origin : '*')
            ->withHeader('Cache-Control', 'public, max-age=300');
    }
}
