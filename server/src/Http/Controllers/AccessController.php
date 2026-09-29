<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Codes\CodeRules;
use Scutoons\Domain\Codes\CodeService;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;

/**
 * Public code lookups (per-IP limit: RateLimit middleware, ACCESS_MAX_PER_IP_HOUR):
 *   GET|POST /api/access/  { code } or ?c= (optional Bearer) → device access for anyone holding the code
 *   GET /api/code/?c=      public info about a code (never an email)
 */
final class AccessController
{
    public function __construct(private readonly CodeService $codes)
    {
    }

    public function access(Request $request, Response $response): Response
    {
        $input = $request->getMethod() === 'POST' ? JsonBody::of($request) : [];
        $query = $request->getQueryParams();
        $code = CodeRules::normalize($input['code'] ?? $input['c'] ?? $query['c'] ?? $query['code'] ?? null)
            ?? throw new ApiError(400, 'invalid_code', 'Code missing or malformed.');
        $toon = $input['toon'] ?? $query['toon'] ?? null;
        $user = CurrentSession::of($request)->user();
        return Json::response($response, 200, $this->codes->access($code, $user, is_string($toon) ? $toon : null));
    }

    public function code(Request $request, Response $response): Response
    {
        $query = $request->getQueryParams();
        $code = CodeRules::normalize($query['c'] ?? $query['code'] ?? null)
            ?? throw new ApiError(400, 'invalid_code', 'Code missing or malformed.');
        return Json::response($response, 200, $this->codes->info($code, CurrentSession::of($request)->user()));
    }
}
