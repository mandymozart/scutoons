<?php
declare(strict_types=1);

namespace Scutoons\Http;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Content\EntityStore;

/**
 * `?only=version` on an item's GET (docs/API.md "Is there a newer version"): the newest version's id alone, `ETag: "<id>"`,
 * `If-None-Match` with it → 304 and no body. One answer for the creator and the deck (RULES H19); each route keeps its
 * own read rights – the caller checks them before.
 */
final class NewestVersion
{
    public static function asked(Request $request): bool
    {
        return ($request->getQueryParams()['only'] ?? '') === 'version';
    }

    public static function answer(Request $request, Response $response, EntityStore $store, string $key): Response
    {
        $id = $store->newestId($key) ?? throw new ApiError(404, 'not_found', 'No version stored yet.');
        $etag = '"' . $id . '"';
        $response = $response->withHeader('ETag', $etag)->withHeader('Access-Control-Expose-Headers', 'ETag');
        $asked = array_map('trim', explode(',', $request->getHeaderLine('If-None-Match')));
        if (in_array($etag, $asked, true) || in_array("W/$etag", $asked, true)) {
            return $response->withStatus(304)->withHeader('Cache-Control', 'private, no-cache');
        }
        return Json::response($response, 200, ['status' => 'success', 'key' => $key, 'version' => $id], ['Cache-Control' => 'private, no-cache']);
    }
}
