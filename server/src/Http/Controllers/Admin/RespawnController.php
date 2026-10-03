<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers\Admin;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Catalog\ItemRespawn;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;

/**
 * /api/admin/catalog/respawn – role admin: when picked-up placed items reappear, per item (Domain\Catalog\ItemRespawn).
 *   GET  /catalog/respawn          → { status, items: [{ id, file, override, overridden, effective }], updated_at, updated_by }
 *   PUT  /catalog/respawn/<item>   { respawn: { default, min?, max? } | null } (null = never reappears)
 *                                  or { reset: true } (back to config/items.json) → { status, changed, …same }
 */
final class RespawnController
{
    public function __construct(private readonly ItemRespawn $respawn)
    {
    }

    public function get(Request $request, Response $response): Response
    {
        return Json::response($response, 200, ['status' => 'success'] + $this->respawn->state());
    }

    /** @param array{item: string} $args */
    public function put(Request $request, Response $response, array $args): Response
    {
        $body = JsonBody::of($request);
        $reset = ($body['reset'] ?? false) === true;
        if (!$reset && !array_key_exists('respawn', $body)) {
            throw new ApiError(400, 'invalid_respawn', 'Send { respawn: { default, min?, max? } | null } or { reset: true }.');
        }
        $admin = $request->getAttribute('user');
        $by = 'admin:' . (is_array($admin) ? (string) ($admin['email'] ?? '') : '');
        $changed = $this->respawn->update($args['item'], $body['respawn'] ?? null, $reset, $by);
        return Json::response($response, 200, ['status' => 'success', 'changed' => $changed] + $this->respawn->state());
    }
}
