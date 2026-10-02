<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\Ownership\OwnershipClaims;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;

/**
 * Ownership claims of the signed-in reader (agents/ITEMS.md "Ownership, claims, gifting"):
 *   POST /api/claims/                      { code, statement } → 201 { status, claim }
 *   GET  /api/claims/                      → { status, items: [claim] } (mine, newest first)
 *   POST /api/claims/{id}/files?name=      the raw file as the body (image/jpeg|png|webp, application/pdf) → { status, claim }
 *   POST /api/claims/{id}/withdraw         → { status, claim }
 * claim = { id, code, status: open|transferred|rejected|withdrawn, statement, files: [{ id, name, type, size }], created_at, decided_at }
 */
final class ClaimsController
{
    public function __construct(private readonly OwnershipClaims $claims)
    {
    }

    public function create(Request $request, Response $response): Response
    {
        $body = JsonBody::of($request);
        $user = CurrentSession::of($request)->requireUser();
        return Json::response($response, 201, ['status' => 'success', 'claim' => $this->claims->file($user, $body)]);
    }

    public function list(Request $request, Response $response): Response
    {
        $user = CurrentSession::of($request)->requireUser();
        return Json::response($response, 200, ['status' => 'success', 'items' => $this->claims->mine((string) $user['id'])]);
    }

    /** @param array{id: string} $args */
    public function addFile(Request $request, Response $response, array $args): Response
    {
        $user = CurrentSession::of($request)->requireUser();
        $name = (string) ($request->getQueryParams()['name'] ?? '');
        return Json::response($response, 200, ['status' => 'success', 'claim' => $this->claims->addFile($user, $args['id'], JsonBody::raw($request), $name)]);
    }

    /** @param array{id: string} $args */
    public function withdraw(Request $request, Response $response, array $args): Response
    {
        $user = CurrentSession::of($request)->requireUser();
        return Json::response($response, 200, ['status' => 'success', 'claim' => $this->claims->withdraw((string) $user['id'], $args['id'])]);
    }
}
