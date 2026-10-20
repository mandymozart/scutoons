<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Domain\Storyboard\Generations;
use Scutoons\Domain\Storyboard\WorkStorage;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Download;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;
use Scutoons\Http\RawUpload;

/**
 * The storyboard's server data beside its document `storyboard:<toon>:<chapter>` (which is an entity: /api/entities) –
 * #120, docs/API.md "Storyboard". Content editors (creator, admin); an AI connection may read and write all of it
 * (only the admin's emptying is refused – ApiTokenScope's /admin rule).
 *   GET  /api/storyboard/generations?toon=&chapter=[&shot=]   → { status, items: [generation] } oldest first
 *   POST /api/storyboard/generations      { toon, chapter, shot, kind, generator, file, machine, … } → 201 { status, item }
 *                                         (a finished local / uploaded generation; provider jobs: #121)
 *   GET  /api/storyboard/generations/{id} → { status, item }
 *   PUT  /api/storyboard/generations/{id} { mark?, approved?, dataset?, note?, machine?, work_path? } → { status, item }
 *   PUT  /api/storyboard/work?toon=&chapter=&name=   raw body = a png / jpeg / webp / mp4 / webm → 201 { status, file }
 *                                         (the same bytes again: 200, `created: false`; other bytes: 409 exists)
 *   GET  /api/storyboard/work?toon=&chapter=          → { status, items: [file] }
 *   GET  /api/storyboard/work/{toon}/{chapter}/{name} the file (Range for video; never cached, never public)
 *   DELETE /api/admin/storyboard/work/{toon}/{chapter} (admin) → { status, deleted, bytes } – after the chapter's release
 */
final class StoryboardController
{
    public function __construct(private readonly Generations $generations, private readonly WorkStorage $work, private readonly Roles $roles)
    {
    }

    /** @return array<string, mixed> the content editor; a write also refuses a cookie from elsewhere (CSRF) */
    private function editor(Request $request, bool $write): array
    {
        $session = CurrentSession::of($request);
        $user = $this->roles->requireContentEditor($session->user());
        if ($write) $session->rejectCsrf();
        return $user;
    }

    public function list(Request $request, Response $response): Response
    {
        $this->editor($request, false);
        $q = $request->getQueryParams();
        $items = $this->generations->list($q['toon'] ?? null, $q['chapter'] ?? null, $q['shot'] ?? null);
        return Json::response($response, 200, ['status' => 'success', 'items' => $items]);
    }

    /** @param array{id: string} $args */
    public function get(Request $request, Response $response, array $args): Response
    {
        $this->editor($request, false);
        return Json::response($response, 200, ['status' => 'success', 'item' => $this->generations->get((int) $args['id'])]);
    }

    public function record(Request $request, Response $response): Response
    {
        $body = JsonBody::of($request);
        $user = $this->editor($request, true);
        $item = $this->generations->record($user, CurrentSession::of($request)->apiToken(), $body);
        return Json::response($response, 201, ['status' => 'success', 'item' => $item]);
    }

    /** @param array{id: string} $args */
    public function update(Request $request, Response $response, array $args): Response
    {
        $body = JsonBody::of($request);
        $this->editor($request, true);
        return Json::response($response, 200, ['status' => 'success', 'item' => $this->generations->update((int) $args['id'], $body)]);
    }

    public function putWork(Request $request, Response $response): Response
    {
        $this->editor($request, true);
        $q = $request->getQueryParams();
        WorkStorage::check($q['toon'] ?? null, $q['chapter'] ?? null, $q['name'] ?? null);
        $tmp = RawUpload::toTempFile($request, $this->work->maxBytes(), 'sb-work-');
        try {
            $file = $this->work->put($q['toon'] ?? null, $q['chapter'] ?? null, $q['name'] ?? null, $tmp, $request->getHeaderLine('Content-Type'));
        } finally {
            @unlink($tmp);
        }
        return Json::response($response, $file['created'] ? 201 : 200, ['status' => 'success', 'file' => $file]);
    }

    public function listWork(Request $request, Response $response): Response
    {
        $this->editor($request, false);
        $q = $request->getQueryParams();
        return Json::response($response, 200, ['status' => 'success', 'items' => $this->work->list($q['toon'] ?? null, $q['chapter'] ?? null)]);
    }

    /** @param array{toon: string, chapter: string, name: string} $args */
    public function getWork(Request $request, Response $response, array $args): Response
    {
        $this->editor($request, false);
        $file = $this->work->file($args['toon'], $args['chapter'], rawurldecode($args['name']));
        return Download::inline($response, $file['path'], $file['type'], $request->getHeaderLine('Range'));
    }

    /** @param array{toon: string, chapter: string} $args */
    public function deleteWork(Request $request, Response $response, array $args): Response
    {
        $result = $this->work->deleteChapter($args['toon'], $args['chapter']);
        return Json::response($response, 200, ['status' => 'success'] + $result);
    }
}
