<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\Embassy\Stockists;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\Cors;
use Scutoons\Support\Json as JsonEncoding;

/**
 * GET /api/stockists/?product=&country= – the public "Where to buy" list (scutoons.com/stores, agents/EMBASSY.md
 * "Stores"): `{ stores: [{ id, name, type, independent, city, country, address, website, opening, carries }],
 * products: { <product id>: <stores> }, countries: [..] }`. No session, the same answer for everyone: any origin may
 * read it (`*` outside ALLOWED_ORIGINS), `Cache-Control: public, max-age=300` + ETag (304), a per-IP limit
 * (config/routes.php). Rules: Domain\Embassy\Stockists.
 */
final class StockistsController
{
    private const MAX_AGE = 300;

    public function __construct(private readonly Stockists $stockists, private readonly Cors $cors)
    {
    }

    public function list(Request $request, Response $response): Response
    {
        $result = $this->stockists->list($request->getQueryParams());
        $body = JsonEncoding::encode(['stores' => $result['stores'], 'products' => (object) $result['products'], 'countries' => $result['countries']]);
        $etag = '"' . substr(hash('sha256', $body), 0, 32) . '"';
        $origin = rtrim($request->getHeaderLine('Origin'), '/');
        $response = $response
            ->withHeader('Content-Type', 'application/json; charset=utf-8')
            ->withHeader('Cache-Control', 'public, max-age=' . self::MAX_AGE)
            ->withHeader('ETag', $etag)
            ->withHeader('Access-Control-Allow-Origin', $origin !== '' && $this->cors->isAllowed($origin) ? $origin : '*')
            ->withHeader('Access-Control-Expose-Headers', 'ETag');
        if (trim($request->getHeaderLine('If-None-Match')) === $etag) return $response->withStatus(304);
        return Json::raw($response, 200, $body, 'application/json; charset=utf-8');
    }
}
