<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Content\EntityKey;
use Scutoons\Domain\Content\EntityStore;
use Scutoons\Domain\Live\LiveSessions;
use Scutoons\Domain\Live\Shows;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;
use Scutoons\Http\NewestVersion;
use stdClass;

/**
 * Show documents and live sessions, role `live` (#90, docs/API.md "Live"; the entity store's other routes refuse `show:` keys):
 *   GET  /api/shows                      every show, the newest draft of each
 *   GET  /api/shows/published[?toon=]    the published shows whose home (first of `toons`) it is (public, like the published scenes: a deck plays one on its
 *                                        own); never a draft
 *   GET  /api/shows/<id>[?draft=1]       published / newest (wrapped); ?v=<n>: the document at that version; ?draft=1&at=<id>:
 *                                        the stored version <id> (wrapped); ?only=version: the newest version's id, ETag / 304
 *   GET  /api/shows/<id>/versions?limit=
 *   PUT  /api/shows/<id> { content, base?, message? }  a new draft; the server stamps `version`
 *   POST /api/shows/<id>/publish { version? }   after the core's check of the whole tree
 *   POST /api/shows/<id>/revert { version, content, base } · /revert-to { version, base }   the History (#687), stamped
 *   POST /api/live/sessions { show? }    a code and the relay's session tokens
 */
final class ShowsController
{
    public function __construct(private readonly EntityStore $store, private readonly Shows $shows, private readonly LiveSessions $sessions, private readonly Roles $roles)
    {
    }

    public function index(Request $request, Response $response): Response
    {
        $this->live($request);
        return Json::response($response, 200, ['status' => 'success', 'shows' => $this->store->withContent(['show'], true)]);
    }

    public function published(Request $request, Response $response): Response
    {
        $toon = $request->getQueryParams()['toon'] ?? null;
        $rows = array_filter($this->store->withContent(['show'], false), fn(array $r) => $r['content'] instanceof stdClass
            && (!is_string($toon) || $toon === '' || ($r['content']->toons[0] ?? null) === $toon));
        return Json::response($response, 200, ['status' => 'success', 'shows' => array_values(array_map(fn(array $r) => ['key' => $r['key'], 'content' => $r['content']], $rows))]);
    }

    /** @param array{id: string} $args */
    public function get(Request $request, Response $response, array $args): Response
    {
        $this->live($request);
        if (NewestVersion::asked($request)) return NewestVersion::answer($request, $response, $this->store, (string) EntityKey::parse('show:' . $args['id']));
        $query = $request->getQueryParams();
        if (isset($query['v'])) {
            $doc = $this->shows->atVersion($args['id'], (int) $query['v']);
            if ($doc === null) throw new ApiError(404, 'unknown_version', 'No such version of this show.');
            return Json::response($response, 200, ['status' => 'success', 'show' => $doc]);
        }
        $key = (string) EntityKey::parse('show:' . $args['id']);
        return Json::response($response, 200, $this->store->wrapped($this->store->read($key, ($query['draft'] ?? '') === '1', (int) ($query['at'] ?? 0))));
    }

    /** @param array{id: string} $args */
    public function versions(Request $request, Response $response, array $args): Response
    {
        $this->live($request);
        $limit = (int) ($request->getQueryParams()['limit'] ?? 100);
        return Json::response($response, 200, $this->store->versions((string) EntityKey::parse('show:' . $args['id']), $limit));
    }

    /** @param array{id: string} $args */
    public function put(Request $request, Response $response, array $args): Response
    {
        $user = $this->live($request);
        return Json::response($response, 200, $this->shows->save($user, EntityKey::parse('show:' . $args['id']), $this->body($request)));
    }

    /** @param array{id: string} $args */
    public function publish(Request $request, Response $response, array $args): Response
    {
        $this->live($request);
        $body = $this->body($request);
        return Json::response($response, 200, $this->shows->publish((string) EntityKey::parse('show:' . $args['id']), isset($body->version) ? (int) $body->version : null));
    }

    /** @param array{id: string, action: string} $args */
    public function revert(Request $request, Response $response, array $args): Response
    {
        $user = $this->live($request);
        $key = EntityKey::parse('show:' . $args['id']);
        $body = $this->body($request);
        $result = $args['action'] === 'revert' ? $this->store->revert($user, $key, $body, 'show') : $this->store->revertTo($user, $key, $body, 'show');
        return Json::response($response, 200, $result);
    }

    public function open(Request $request, Response $response): Response
    {
        $this->live($request);
        $show = $this->body($request)->show ?? null;
        return Json::response($response, 200, ['status' => 'success'] + $this->sessions->open(is_string($show) ? $show : null));
    }

    /** @return array<string, mixed> */
    private function live(Request $request): array
    {
        $session = CurrentSession::of($request);
        $session->rejectCsrf();
        return $this->roles->require($session->user(), Roles::LIVE);
    }

    private function body(Request $request): stdClass
    {
        $body = JsonBody::object($request);
        return $body instanceof stdClass ? $body : new stdClass();
    }
}
