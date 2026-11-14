<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Content\FlowIndex;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Json;

/**
 * The creator's SCUTOONS-wide flow (owner 2026-10-06, agents/DESIGN.md "Flow panel"; role creator):
 *   GET /api/creator/flow-index   { toons: [{ id, title, status, seasons }], edges: [{ source, target, kind, label? }], dead }
 * from the newest versions (drafts included) – Domain\Content\FlowIndex.
 *   GET /api/creator/story-events { status, toons: [{ id, chapters: [{ number, events: [{ id|null, type, characters, excerpt|null, group|null, groupIndex }],
 *                                 choices: [{ event|null, group|null, options: [{ id|null, label: { <locale>: text } }], version }] }] }] }
 * the story events every toon's chapters fire (#521) and the questions of their published versions (#550) – FlowIndex::storyEvents;
 * read by the content editors and the role `live` (#555, owner 2026-11-14: a show's rows fire on them – the deck editor's account is `live`).
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

    public function storyEvents(Request $request, Response $response): Response
    {
        $user = CurrentSession::of($request)->requireUser();
        if (!$this->roles->canEditContent($user) && !$this->roles->has($user, Roles::LIVE)) {
            throw new ApiError(403, 'forbidden', 'The story events are for the content editors (creator, admin) and live.', ['role' => Roles::LIVE]);
        }
        return Json::response($response, 200, $this->index->storyEvents(), ['Cache-Control' => 'no-store']);
    }
}
