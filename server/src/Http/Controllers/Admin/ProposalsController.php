<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers\Admin;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\Admin\Proposals;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;
use Scutoons\Infrastructure\Config\Settings;

/**
 * /api/admin/proposals – role admin (docs/API.md "Admin API", agents/MCP.md "Draft → link"): changes an admin's own AI
 * proposes, applied by the admin in the admin app (Domain\Admin\Proposals).
 *   GET  /proposals?status=open|applied|dismissed → { status, items: [{ id, title, count, status, author, created_at, … }] }
 *   POST /proposals        { title, changes, calendar_base?, source? } → { status, proposal, url } (201)
 *   GET  /proposals/{id}   → { status, proposal, url }
 *   PUT  /proposals/{id}   { status: applied | dismissed } → { status, proposal } (409 decided: not open any more)
 */
final class ProposalsController
{
    public function __construct(private readonly Proposals $proposals, private readonly Settings $settings)
    {
    }

    public function list(Request $request, Response $response): Response
    {
        $status = $request->getQueryParams()['status'] ?? null;
        return Json::response($response, 200, ['status' => 'success', 'items' => $this->proposals->list(is_string($status) && $status !== '' ? $status : null)]);
    }

    public function create(Request $request, Response $response): Response
    {
        $proposal = $this->proposals->create(JsonBody::of($request), $this->admin($request));
        return Json::response($response, 201, ['status' => 'success', 'proposal' => $proposal, 'url' => $this->url((string) $proposal['id'])]);
    }

    /** @param array{id: string} $args */
    public function get(Request $request, Response $response, array $args): Response
    {
        return Json::response($response, 200, ['status' => 'success', 'proposal' => $this->proposals->get($args['id']), 'url' => $this->url($args['id'])]);
    }

    /** @param array{id: string} $args */
    public function decide(Request $request, Response $response, array $args): Response
    {
        $status = (string) (JsonBody::of($request)['status'] ?? '');
        return Json::response($response, 200, ['status' => 'success', 'proposal' => $this->proposals->decide($args['id'], $status, $this->admin($request))]);
    }

    /** @return array<string, mixed> */
    private function admin(Request $request): array
    {
        $user = $request->getAttribute('user');
        return is_array($user) ? $user : [];
    }

    private function url(string $id): string
    {
        return rtrim($this->settings->get('ADMIN_URL'), '/') . '/proposals/' . rawurlencode($id);
    }
}
