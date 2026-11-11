<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Content\ContentSchemas;
use Scutoons\Domain\Content\EntityKey;
use Scutoons\Domain\Content\EntityStore;
use Scutoons\Domain\Live\Scenes;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;
use stdClass;

/**
 * Deck scenes `scene:<id>` (#243, research/SCENES.md §3, docs/API.md "Scenes"; the entity store's other routes refuse
 * `scene:` keys). Writing: the content editors (creator, admin), as every content document. Reading a PUBLISHED scene:
 * everyone, like the catalog and scene-content – a reader's browser deck needs it, readers save nothing (proposal,
 * SCENES.md §7 question 1); drafts and the history: the content editors only (a draft never leaves the editor).
 *   GET  /api/scenes                         the published scenes (public); a content editor: every scene, its newest draft
 *   GET  /api/scenes/<id>                    the published one { status, scene } (public)
 *   GET  /api/scenes/<id>?v=<n>              the document at version <n> { status, scene } – public only the published one
 *   GET  /api/scenes/<id>?draft=1            the newest, wrapped { status, version, published, content } (content editors)
 *   GET  /api/scenes/<id>/versions?limit=    the history (content editors)
 *   PUT  /api/scenes/<id> { content, base?, message? }   a new draft (422 invalid_content: not the scene's shape); the
 *                                            server stamps `version`
 *   POST /api/scenes/<id>/publish { version? }   after the core's check (422 invalid_scene { problems }); never with an
 *                                            AI connection's token (ApiTokenScope, RULES G2)
 *   POST /api/scenes/<id>/restore { id, message? }   an older version as the newest draft
 */
final class ScenesController
{
    public function __construct(private readonly EntityStore $store, private readonly Scenes $scenes, private readonly ContentSchemas $schemas, private readonly Roles $roles)
    {
    }

    public function index(Request $request, Response $response): Response
    {
        $editor = $this->editor($request) !== null;
        $rows = $this->store->withContent(['scene'], $editor);
        if (!$editor) {
            // what a reader may see of a published scene: its document; who saved it stays in the editor's history
            $rows = array_values(array_map(fn(array $r) => ['key' => $r['key'], 'content' => $r['content']], array_filter($rows, fn(array $r) => $r['content'] !== null)));
        }
        return Json::response($response, 200, ['status' => 'success', 'scenes' => $rows]);
    }

    /** @param array{id: string} $args */
    public function get(Request $request, Response $response, array $args): Response
    {
        $key = $this->key($args['id']);
        $query = $request->getQueryParams();
        if (($query['draft'] ?? '') === '1') {
            $this->requireEditor($request);
            return Json::response($response, 200, $this->store->wrapped($this->store->read((string) $key, true, 0)));
        }
        $published = $this->scenes->published((string) $key->id);
        if (isset($query['v'])) {
            $v = (int) $query['v'];
            $doc = $this->editor($request) !== null ? $this->scenes->atVersion((string) $key->id, $v) : ($published !== null && ($published->version ?? null) === $v ? $published : null);
            if ($doc === null) throw new ApiError(404, 'unknown_version', 'No such version of this scene.');
            return Json::response($response, 200, ['status' => 'success', 'scene' => $doc]);
        }
        if ($published === null) throw new ApiError(404, 'not_found', 'Not published.');
        return Json::response($response, 200, ['status' => 'success', 'scene' => $published]);
    }

    /** @param array{id: string} $args */
    public function versions(Request $request, Response $response, array $args): Response
    {
        $this->requireEditor($request);
        $limit = (int) ($request->getQueryParams()['limit'] ?? 100);
        return Json::response($response, 200, $this->store->versions((string) $this->key($args['id']), $limit));
    }

    /** @param array{id: string} $args */
    public function put(Request $request, Response $response, array $args): Response
    {
        $user = $this->requireEditor($request);
        $key = $this->key($args['id']);
        $body = $this->body($request);
        if (property_exists($body, 'content')) $this->schemas->guard($key, $body->content);
        $ai = CurrentSession::of($request)->apiToken();
        if ($ai !== null) $body->message = mb_substr(trim((is_string($body->message ?? null) ? $body->message : '') . " (AI: {$ai['name']})"), 0, 255);
        return Json::response($response, 200, $this->scenes->save($user, $key, $body));
    }

    /** @param array{id: string} $args */
    public function publish(Request $request, Response $response, array $args): Response
    {
        $this->requireEditor($request);
        $body = $this->body($request);
        return Json::response($response, 200, $this->scenes->publish($this->key($args['id']), isset($body->version) ? (int) $body->version : null));
    }

    /** @param array{id: string} $args */
    public function restore(Request $request, Response $response, array $args): Response
    {
        $user = $this->requireEditor($request);
        return Json::response($response, 200, $this->scenes->restore($user, $this->key($args['id']), $this->body($request)));
    }

    private function key(string $id): EntityKey
    {
        return EntityKey::parse("scene:$id");
    }

    /** @return array<string, mixed>|null the signed-in content editor, or null (a reader, no sign-in) */
    private function editor(Request $request): ?array
    {
        $session = CurrentSession::of($request);
        if ($session->csrfBlocked()) return null;
        $user = $session->user();
        return $user !== null && $this->roles->canEditContent($user) ? $user : null;
    }

    /** @return array<string, mixed> */
    private function requireEditor(Request $request): array
    {
        $session = CurrentSession::of($request);
        $session->rejectCsrf();
        return $this->roles->requireContentEditor($session->user());
    }

    /** The request body decoded with objects; none (or not an object) = `{}`, as for shows */
    private function body(Request $request): stdClass
    {
        if (strlen(JsonBody::raw($request)) > $this->store->maxBytes() + 65536) {
            throw new ApiError(413, 'content_too_large', 'The document is larger than ' . $this->store->maxBytes() . ' bytes.');
        }
        $body = JsonBody::raw($request) === '' ? null : JsonBody::object($request);
        return $body instanceof stdClass ? $body : new stdClass();
    }
}
