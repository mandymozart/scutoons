<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Codes\CodeRules;
use Scutoons\Domain\Codes\CodeService;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;
use Scutoons\Infrastructure\Clock\Clock;

/**
 * PUBLIC URL – printed/linked, must keep working unchanged (agents/PLATFORM.md "RULE: printed / public URLs never
 * change"). Legacy membership validation for old viewer builds: POST { code } / { c } or GET ?code= / ?c= (no login).
 *   → 200 { status: "success", qr: { …row without email / signature / claimed_by / revoked_*, is_claimed, is_expired } }
 *   400 invalid_code · 404 not_found · 410 expired / revoked. CORS `*` (Cors middleware).
 * Success for any existing, unexpired, unrevoked code – claimed or not (viewing is device-level).
 */
final class ValidateController
{
    public function __construct(private readonly CodeService $codes, private readonly Clock $clock)
    {
    }

    public function __invoke(Request $request, Response $response): Response
    {
        $method = $request->getMethod();
        if ($method !== 'GET' && $method !== 'POST') throw new ApiError(405, 'method_not_allowed', 'Use GET or POST.');

        $input = $method === 'POST' ? JsonBody::of($request) : [];
        $query = $request->getQueryParams();
        $code = CodeRules::normalize($input['code'] ?? $input['c'] ?? $query['code'] ?? $query['c'] ?? null)
            ?? throw new ApiError(400, 'invalid_code', 'QR code parameter is required');

        $row = $this->codes->find($code) ?? throw new ApiError(404, 'not_found', 'QR code not found');
        if (CodeRules::isExpired($row, $this->clock->time())) throw new ApiError(410, 'expired', 'QR code expired');
        if (CodeRules::isRevoked($row)) throw new ApiError(410, 'revoked', 'QR code is no longer valid');

        $isClaimed = CodeRules::isClaimed($row);
        unset($row['email'], $row['signature'], $row['claimed_by'], $row['revoked_at'], $row['revoked_reason']);
        foreach (['created_at', 'claimed_at'] as $field) {
            if (!empty($row[$field])) $row[$field] = date('c', (int) strtotime((string) $row[$field]));
        }
        $row['is_claimed'] = $isClaimed;
        $row['is_expired'] = false;
        return Json::response($response, 200, ['status' => 'success', 'qr' => $row]);
    }
}
