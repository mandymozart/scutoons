<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Catalog\Catalog;
use Scutoons\Domain\Catalog\Editions;
use Scutoons\Domain\Catalog\Toons;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\Cors;

/**
 * Public data, identical for everyone:
 *   GET /api/items/     the item catalog (config/items.json)
 *   GET /api/editions/  the editions (config/editions.json), ?toon= filters
 *   GET /api/toons/     the toons (config/toons.json) – no CORS, any method (as before)
 * items/editions: any origin may read them (`*` for origins outside ALLOWED_ORIGINS), cached 5 minutes.
 */
final class CatalogController
{
    public function __construct(
        private readonly Catalog $catalog,
        private readonly Editions $editions,
        private readonly Toons $toons,
        private readonly Cors $cors,
    ) {
    }

    public function items(Request $request, Response $response): Response
    {
        self::requireGet($request);
        return $this->public($request, $response, $this->catalog->data());
    }

    public function editions(Request $request, Response $response): Response
    {
        self::requireGet($request);
        $toon = $request->getQueryParams()['toon'] ?? '';
        $editions = $this->editions->all();
        if ($toon !== '') {
            $editions = array_values(array_filter($editions, fn(array $edition) => ($edition['toon'] ?? null) === $toon));
        }
        return $this->public($request, $response, $editions);
    }

    public function toons(Request $request, Response $response): Response
    {
        // As served since the first version: default JSON flags, no charset, no caching headers
        $response->getBody()->write((string) json_encode($this->toons->all()));
        return $response->withHeader('Content-Type', 'application/json');
    }

    /** The data files themselves under their old URLs (/api/items/items.json, …) */
    public function file(Request $request, Response $response): Response
    {
        $path = match (basename($request->getUri()->getPath())) {
            'items.json' => $this->catalog->path(),
            'editions.json' => $this->editions->path(),
            default => $this->toons->path(),
        };
        return Json::raw($response, 200, (string) file_get_contents($path), 'application/json');
    }

    private static function requireGet(Request $request): void
    {
        if ($request->getMethod() !== 'GET') {
            throw new ApiError(405, 'method_not_allowed', 'Use GET.');
        }
    }

    private function public(Request $request, Response $response, mixed $data): Response
    {
        $origin = rtrim($request->getHeaderLine('Origin'), '/');
        return Json::response($response, 200, $data, [
            'Access-Control-Allow-Origin' => $origin !== '' && $this->cors->isAllowed($origin) ? $origin : '*',
            'Cache-Control' => 'public, max-age=300',
        ]);
    }
}
