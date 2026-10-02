<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers\Admin;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\Admin\ListQuery;
use Scutoons\Domain\Ownership\OwnershipClaims;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;
use Scutoons\Infrastructure\Config\Settings;

/**
 * /api/admin/claims – ownership claims, role admin (docs/API.md "Admin API" → Claims):
 *   GET  /claims                          list (status, code, user, q; sort created_at, decided_at, status, code) + `open` (count)
 *   GET  /claims/{id}                     claim + code + current owner + certificate check + history + other claims
 *   POST /claims/{id}/transfer { note }   move the code to the claimant (mails both owners)
 *   POST /claims/{id}/reject   { note }   keep it (mails the claimant)
 *   GET  /claims/{id}/files/{fileId}      an evidence file (inline, no-store, nosniff)
 */
final class ClaimsController
{
    public function __construct(private readonly OwnershipClaims $claims, private readonly Settings $settings)
    {
    }

    public function list(Request $request, Response $response): Response
    {
        $query = ListQuery::from($request->getQueryParams(), OwnershipClaims::SORTABLE, '-created_at', OwnershipClaims::FILTERS, 50, $this->settings->int('ADMIN_MAX_PAGE_SIZE'));
        return Json::response($response, 200, ['status' => 'success'] + $this->claims->list($query) + ['open' => $this->claims->openCount()]);
    }

    /** @param array{id: string} $args */
    public function get(Request $request, Response $response, array $args): Response
    {
        return Json::response($response, 200, ['status' => 'success'] + $this->claims->detail($args['id']));
    }

    /** @param array{id: string} $args */
    public function transfer(Request $request, Response $response, array $args): Response
    {
        $body = JsonBody::of($request);
        $admin = CurrentSession::of($request)->requireUser();
        return Json::response($response, 200, ['status' => 'success'] + $this->claims->transfer($admin, $args['id'], $body));
    }

    /** @param array{id: string} $args */
    public function reject(Request $request, Response $response, array $args): Response
    {
        $body = JsonBody::of($request);
        $admin = CurrentSession::of($request)->requireUser();
        return Json::response($response, 200, ['status' => 'success'] + $this->claims->reject($admin, $args['id'], $body));
    }

    /** @param array{id: string, file: string} $args */
    public function file(Request $request, Response $response, array $args): Response
    {
        [$bytes, $type, $name] = $this->claims->adminFile($args['id'], $args['file']);
        $response->getBody()->write($bytes);
        return $response->withStatus(200)
            ->withHeader('Content-Type', $type)
            ->withHeader('Content-Length', (string) strlen($bytes))
            ->withHeader('Content-Disposition', 'inline; filename="' . str_replace('"', '', $name) . '"')
            ->withHeader('X-Content-Type-Options', 'nosniff')
            ->withHeader('Content-Security-Policy', "default-src 'none'; img-src 'self'; style-src 'unsafe-inline'; sandbox")
            ->withHeader('Cache-Control', 'no-store');
    }
}
