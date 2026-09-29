<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers\Admin;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\Settings\RuntimeSettings;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;

/**
 * /api/admin/settings – role admin (docs/API.md "Admin API"): the runtime settings (Domain\Settings\RuntimeSettings).
 *   GET  /settings   → { status, items: [{ key, value, default, min, max, stored, updated_at, updated_by }] }
 *   PUT  /settings   { key: int|null, … } → { status, changed: [keys], items } (null = back to the default)
 */
final class SettingsController
{
    public function __construct(private readonly RuntimeSettings $settings)
    {
    }

    public function get(Request $request, Response $response): Response
    {
        return Json::response($response, 200, ['status' => 'success', 'items' => $this->settings->all()]);
    }

    public function put(Request $request, Response $response): Response
    {
        $body = JsonBody::of($request);
        $values = is_array($body['settings'] ?? null) ? $body['settings'] : $body;
        $admin = $request->getAttribute('user');
        $by = 'admin:' . (is_array($admin) ? (string) ($admin['email'] ?? '') : '');
        $changed = $this->settings->update($values, $by);
        return Json::response($response, 200, ['status' => 'success', 'changed' => $changed, 'items' => $this->settings->all()]);
    }
}
