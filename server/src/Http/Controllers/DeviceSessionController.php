<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\Layouts\DeviceSessions;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;

/**
 * The creator's resume state per device (Domain\Layouts\DeviceSessions) – signed in, own rows only; an AI connection only reads:
 *   GET /api/me/session?device=[&app=]  { available, mine: { device, name, state, updated_at } | null, other: … | null[, app: … | null] }
 *   PUT /api/me/session          { device, name, state } → { session: row }
 */
final class DeviceSessionController
{
    public function __construct(private readonly DeviceSessions $sessions)
    {
    }

    public function mine(Request $request, Response $response): Response
    {
        $user = CurrentSession::of($request)->requireUser();
        $query = $request->getQueryParams();
        $device = $query['device'] ?? '';
        $app = $query['app'] ?? null;
        return Json::response($response, 200, ['status' => 'success'] + $this->sessions->of((string) $user['id'], is_string($device) ? $device : '', is_string($app) ? $app : null));
    }

    public function save(Request $request, Response $response): Response
    {
        $body = JsonBody::of($request);
        $user = CurrentSession::of($request)->requireUser();
        return Json::response($response, 200, ['status' => 'success', 'session' => $this->sessions->save((string) $user['id'], $body)]);
    }
}
