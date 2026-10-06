<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\Content\FlowIndex;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Json;

/**
 * The creator's SCUTOONS-wide flow (owner 2026-10-06, agents/DESIGN.md "Flow panel"; role creator):
 *   GET /api/creator/flow-index   { toons: [{ id, title, status, seasons }], edges: [{ source, target, kind, label? }], dead }
 * from the newest versions (drafts included) – Domain\Content\FlowIndex.
 */
final class FlowController
{
    public function __construct(private readonly FlowIndex $index, private readonly Roles $roles)
    {
    }

    public function index(Request $request, Response $response): Response
    {
        $this->roles->requireContentEditor(CurrentSession::of($request)->user());
        return Json::response($response, 200, $this->index->build(), ['Cache-Control' => 'no-store']);
    }
}
