<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Domain\Storyboard\GenerationBudget;
use Scutoons\Domain\Storyboard\GenerationJobs;
use Scutoons\Domain\Storyboard\Generations;
use Scutoons\Domain\Storyboard\Generators\GeneratorRegistry;
use Scutoons\Domain\Storyboard\RefLinks;
use Scutoons\Domain\Storyboard\WorkStorage;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Download;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;
use Scutoons\Http\RawUpload;
use Slim\Psr7\Stream;

/**
 * The storyboard's server data beside its document `storyboard:<toon>:<chapter>` (which is an entity: /api/entities) –
 * #120, docs/API.md "Storyboard". Content editors (creator, admin); an AI connection may read and write all of it
 * (only the admin's emptying is refused – ApiTokenScope's /admin rule).
 *   GET  /api/storyboard/generations?toon=&chapter=[&shot=]   → { status, items: [generation] } oldest first
 *   POST /api/storyboard/generations      { toon, chapter, shot, kind, generator, file, machine, … } → 201 { status, item }
 *                                         (a finished local / uploaded generation) – or a PAID provider job (#121):
 *                                         { toon, chapter, shot, generator, model, prompt?, references?, params?, aspect? }
 *                                         → the budget gate (402), the provider's task started → 201 { status, item }
 *   POST /api/storyboard/generations/estimate  the same job body → { status, estimate } (nothing started)
 *   GET  /api/storyboard/generations/{id} → { status, item } (an open job: the provider asked first, rate-limited)
 *   POST /api/storyboard/generations/{id}/cancel → { status, item }
 *   GET  /api/storyboard/generations/{id}/result  the provider's result passed through (nothing stored)
 *   GET  /api/storyboard/generators       → { status, generators, budget }
 *   GET  /api/storyboard/budget           → { status, budget }
 *   GET  /api/storyboard/ref/{toon}/{chapter}/{name}?expires=&sig=  a work file for a provider (signed, no session)
 *   PUT  /api/storyboard/generations/{id} { mark?, approved?, dataset?, note?, machine?, work_path? } → { status, item }
 *   PUT  /api/storyboard/work?toon=&chapter=&name=   raw body = a png / jpeg / webp / mp4 / webm → 201 { status, file }
 *                                         (the same bytes again: 200, `created: false`; other bytes: 409 exists)
 *   GET  /api/storyboard/work?toon=&chapter=          → { status, items: [file] }
 *   GET  /api/storyboard/work/{toon}/{chapter}/{name} the file (Range for video; never cached, never public)
 *   DELETE /api/admin/storyboard/work/{toon}/{chapter} (admin) → { status, deleted, bytes } – after the chapter's release
 */
final class StoryboardController
{
    public function __construct(
        private readonly Generations $generations,
        private readonly WorkStorage $work,
        private readonly Roles $roles,
        private readonly GenerationJobs $jobs,
        private readonly GeneratorRegistry $registry,
        private readonly GenerationBudget $budget,
        private readonly RefLinks $links,
    ) {
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
        $items = $this->jobs->list($q['toon'] ?? null, $q['chapter'] ?? null, $q['shot'] ?? null);
        return Json::response($response, 200, ['status' => 'success', 'items' => $items]);
    }

    /** @param array{id: string} $args */
    public function get(Request $request, Response $response, array $args): Response
    {
        $this->editor($request, false);
        return Json::response($response, 200, ['status' => 'success', 'item' => $this->jobs->get((int) $args['id'])]);
    }

    public function record(Request $request, Response $response): Response
    {
        $body = JsonBody::of($request);
        $user = $this->editor($request, true);
        $token = CurrentSession::of($request)->apiToken();
        $item = $this->jobs->isJob($body) ? $this->jobs->start($user, $token, $body) : $this->generations->record($user, $token, $body);
        return Json::response($response, 201, ['status' => 'success', 'item' => $item]);
    }

    public function estimate(Request $request, Response $response): Response
    {
        $body = JsonBody::of($request);
        $this->editor($request, false);
        return Json::response($response, 200, ['status' => 'success', 'estimate' => $this->jobs->estimate($body)]);
    }

    /** @param array{id: string} $args */
    public function cancel(Request $request, Response $response, array $args): Response
    {
        $this->editor($request, true);
        return Json::response($response, 200, ['status' => 'success', 'item' => $this->jobs->cancel((int) $args['id'])]);
    }

    /** @param array{id: string} $args */
    public function result(Request $request, Response $response, array $args): Response
    {
        $this->editor($request, false);
        [$stream, $type, $size] = $this->jobs->result((int) $args['id']);
        return $response->withBody(new Stream($stream))->withHeader('Content-Type', $type)->withHeader('Content-Length', (string) $size)
            ->withHeader('Cache-Control', 'private, no-store')->withHeader('X-Content-Type-Options', 'nosniff');
    }

    public function generators(Request $request, Response $response): Response
    {
        $this->editor($request, false);
        return Json::response($response, 200, ['status' => 'success', 'generators' => $this->registry->describe(), 'budget' => $this->budget->state()]);
    }

    public function budget(Request $request, Response $response): Response
    {
        $this->editor($request, false);
        return Json::response($response, 200, ['status' => 'success', 'budget' => $this->budget->state()]);
    }

    /**
     * A work file for a provider: no session – the signed link (RefLinks) is the permission, for that one file, a few
     * minutes. A bad or expired link is a plain 404 (it says nothing about the file).
     * @param array{toon: string, chapter: string, name: string} $args
     */
    public function ref(Request $request, Response $response, array $args): Response
    {
        $q = $request->getQueryParams();
        $name = rawurldecode($args['name']);
        [$toon, $chapter] = WorkStorage::check($args['toon'], $args['chapter']);
        if (!$this->links->valid($toon, $chapter, $name, $q['expires'] ?? null, $q['sig'] ?? null)) throw new ApiError(404, 'not_found', 'No such file.');
        $file = $this->work->file($toon, $chapter, $name);
        return Download::inline($response, $file['path'], $file['type'], $request->getHeaderLine('Range'));
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
