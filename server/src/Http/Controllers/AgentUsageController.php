<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\Agent\AgentApps;
use Scutoons\Domain\Agent\AgentUsage;
use Scutoons\Domain\ApiError;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Support\Json as JsonEncoding;

/**
 * The usage log of the Ask panels (docs/API.md "Agent usage"):
 *   POST /api/agent-usage             the Ask service logs one AI API call: the user's session (Bearer) AND
 *                                     X-Service-Key = AGENT_SERVICE_KEY; the row's user = the session's user, who must
 *                                     be allowed to use the body's `app` (default creator; AgentApps)
 *   GET  /api/admin/agent-usage       ?from=&to=&group=day|month – the admin overview (admin group, RequireRole)
 */
final class AgentUsageController
{
    public const KEY_HEADER = 'X-Service-Key';

    public function __construct(private readonly AgentUsage $usage, private readonly AgentApps $apps, private readonly Settings $settings)
    {
    }

    public function record(Request $request, Response $response): Response
    {
        $this->requireServiceKey($request);
        $session = CurrentSession::of($request);
        $session->rejectCsrf();
        $user = $session->user() ?? throw new ApiError(401, 'unauthorized', 'Not signed in.');
        $body = JsonBody::object($request) ?? throw new ApiError(422, 'invalid_body', 'The request body must be a JSON object { service, model, … }.');
        $app = AgentApps::app($body->app ?? null);
        $user = $this->apps->requireUser($app, $user);
        $this->usage->record((string) $user['id'], $body, $app);
        return Json::response($response, 201, ['status' => 'success']);
    }

    public function overview(Request $request, Response $response): Response
    {
        // cost_usd stays a JSON number with a fraction (0.0, 1.0) for typed clients
        return Json::response($response, 200, ['status' => 'success'] + $this->usage->overview($request->getQueryParams()), [], JsonEncoding::FLAGS | JSON_PRESERVE_ZERO_FRACTION);
    }

    /** 503 not_configured without AGENT_SERVICE_KEY, 403 forbidden for a missing / wrong key */
    private function requireServiceKey(Request $request): void
    {
        $key = $this->settings->get('AGENT_SERVICE_KEY');
        if ($key === '') throw new ApiError(503, 'not_configured', 'The usage log is not configured (AGENT_SERVICE_KEY).');
        $sent = $request->getHeaderLine(self::KEY_HEADER);
        if ($sent === '' || !hash_equals($key, $sent)) throw new ApiError(403, 'forbidden', 'Only the Ask service may write the usage log.');
    }
}
