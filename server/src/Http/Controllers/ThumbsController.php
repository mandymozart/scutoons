<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Domain\Thumbnails\Thumbnails;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Json;
use Scutoons\Http\RawUpload;

/**
 * Thumbnails (Domain\Thumbnails, docs/API.md "Thumbnails") – signed in:
 *   GET /api/thumbs?keys=a,b,…          { available, thumbs: { key: { url, stamp, stale, origin, taken_at } } } (missing keys left out;
 *                                       workspace keys: only the member's own)
 *   PUT /api/thumbs?key=&stamp=         raw body = a 264 × 168 WebP → { thumb: { key, url, … } } (shared keys: content editors;
 *                                       workspace keys: the member's own)
 */
final class ThumbsController
{
    public function __construct(private readonly Thumbnails $thumbnails, private readonly Roles $roles)
    {
    }

    public function lookup(Request $request, Response $response): Response
    {
        $user = CurrentSession::of($request)->requireUser();
        $keys = (string) ($request->getQueryParams()['keys'] ?? '');
        $result = $this->thumbnails->lookup($keys === '' ? [] : explode(',', $keys), (string) $user['id']);
        return Json::response($response, 200, ['status' => 'success'] + $result);
    }

    public function put(Request $request, Response $response): Response
    {
        $session = CurrentSession::of($request);
        $user = $session->requireUser();
        $session->rejectCsrf();
        $query = $request->getQueryParams();
        $key = is_string($query['key'] ?? null) ? $query['key'] : '';
        if (Thumbnails::kindOf($key) !== 'workspace') $this->roles->requireContentEditor($user);
        $stamp = is_string($query['stamp'] ?? null) ? mb_substr($query['stamp'], 0, 100) : null;
        $file = RawUpload::toTempFile($request, Thumbnails::MAX_BYTES, 'thumb-');
        try {
            $thumb = $this->thumbnails->put((string) $user['id'], $key, $file, $stamp);
        } finally {
            @unlink($file);
        }
        return Json::response($response, 200, ['status' => 'success', 'thumb' => $thumb]);
    }
}
