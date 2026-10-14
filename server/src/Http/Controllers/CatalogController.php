<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Catalog\Catalog;
use Scutoons\Domain\Catalog\Editions;
use Scutoons\Domain\Catalog\ProtectedFiles;
use Scutoons\Domain\Catalog\Toons;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\Cors;
use Scutoons\Support\Json as JsonEncoding;

/**
 * Public data, identical for everyone:
 *   GET /api/items/     the item catalog (tables catalog_items / _recipes / _albums + slots / stats from code)
 *   GET /api/editions/  the editions (catalog_editions), ?toon= filters
 *   GET /api/toons/     the toons (table toons) – no CORS, any method (as before)
 * (agents/CATALOG.md: the catalog moved from server/config/*.json into the database; same JSON shapes)
 * items/editions: any origin may read them (`*` for origins outside ALLOWED_ORIGINS), cached 5 minutes.
 */
final class CatalogController
{
    public function __construct(
        private readonly Catalog $catalog,
        private readonly Editions $editions,
        private readonly Toons $toons,
        private readonly Cors $cors,
        private readonly ProtectedFiles $files,
    ) {
    }

    public function items(Request $request, Response $response): Response
    {
        self::requireGet($request);
        $data = $this->catalog->data();
        // a score's file paths stay private: readers ask for signed URLs (scoreFile)
        $data['items'] = array_map([ProtectedFiles::class, 'publicItem'], $data['items']);
        return $this->public($request, $response, $data);
    }

    /**
     * GET /api/items/{id}/score/{file} → { url, expires } – a signed URL of a score's MusicXML / PDF (ProtectedFiles)
     * @param array<string, string> $args
     */
    public function scoreFile(Request $request, Response $response, array $args): Response
    {
        $user = CurrentSession::of($request)->user();
        $part = $request->getQueryParams()['part'] ?? null;
        $signed = $this->files->score((string) ($args['id'] ?? ''), (string) ($args['file'] ?? ''), $user, is_string($part) ? $part : null);
        return Json::response($response, 200, ['status' => 'success'] + $signed, ['Cache-Control' => 'no-store']);
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

    /** The old data file URLs (/api/items/items.json, …): the same data, now from the catalog tables */
    public function file(Request $request, Response $response): Response
    {
        $data = match (basename($request->getUri()->getPath())) {
            'items.json' => $this->catalog->data(),
            'editions.json' => $this->editions->all(),
            default => $this->toons->all(),
        };
        return Json::raw($response, 200, JsonEncoding::encode($data, JsonEncoding::FLAGS | JSON_PRETTY_PRINT), 'application/json');
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
