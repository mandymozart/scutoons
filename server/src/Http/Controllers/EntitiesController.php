<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Assets\AssetUsages;
use Scutoons\Domain\Assets\IdleCheck;
use Scutoons\Domain\Catalog\CatalogDocs;
use Scutoons\Domain\Catalog\PublishDependencies;
use Scutoons\Domain\Catalog\PublishOrder;
use Scutoons\Domain\Live\Scenes;
use Scutoons\Domain\Content\ContentSchemas;
use Scutoons\Domain\Content\EntityKey;
use Scutoons\Domain\Content\EntityStore;
use Scutoons\Domain\Content\FlowCheck;
use Scutoons\Domain\Content\ItemPlacements;
use Scutoons\Domain\Content\MapRefs;
use Scutoons\Domain\Content\Reviews;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;
use Scutoons\Http\NewestVersion;
use stdClass;

/**
 * The creator's write path on the content store, by entity key (docs/API.md "Entities"; role creator for everything –
 * readers use the library endpoints):
 *   GET  /api/entities?toon=                     every stored entity of a toon (`toon=` empty: platform-wide ones)
 *   GET  /api/entities?kinds=place,route,map&content=draft|published   every entity of these kinds WITH its document +
 *                                                `first` (creation order) – a page that edits many documents (the world map)
 *   POST /api/entities/batch { saves: [{ key, content, base?, message? }], deletes?: [key], message? }   several new
 *                                                draft versions in one transaction (all or none; 409 / 422 name the `key`);
 *                                                `deletes`: never-published place / route / catalog keys (409 published otherwise)
 *   POST /api/entities/publish { versions: [{ key, version? }], partial? }   several documents live at once (one transaction);
 *                                                `partial: true`: in dependency order, each unit on its own – refusals named
 *   GET  /api/entities/<key>[?draft=1|?version=<id>][&wrap=1]   published / newest / one version (raw or wrapped)
 *   GET  /api/entities/<key>?only=version         the newest version's id, ETag / 304 (NewestVersion)
 *   PUT  /api/entities/<key>                     { content, message?, base? } → new draft version (409 conflict;
 *                                                422 invalid_content { problems } when it doesn't fit its type: ContentSchemas,
 *                                                or can get the reader stuck: FlowCheck; 422 item_limit { problems }
 *                                                when it adds placements over a catalog item's `placements`)
 *   GET  /api/entities/<key>/versions?limit=     history, newest first
 *   POST /api/entities/<key>/publish { version? } · /unpublish · /restore { id, message? }
 *   POST /api/entities/<key>/revert { version, content, base } · /revert-to { version, base }   the History (#687): one
 *                                                change undone (the core's revert as content) / back to a version – a new draft
 *   GET  /api/catalog/items/<id>/placements      where a catalog item is placed in every toon + its limits and owners
 *                                                (ItemPlacements, BACKLOG 10.39) – 404 unknown_item
 *   GET  /api/catalog/docs/<kind>                every catalog entry of a kind as a document (CatalogDocs::list)
 * publish / unpublish: 403 review_required for accounts that need a review for the toon (Reviews; submit through
 * /api/reviews instead).
 *
 * Catalog documents (`item:<id>`, `recipe:<id>`, `album:<id>`, `product:<id>`, `edition:<id>` – CatalogDocs,
 * agents/CATALOG-IN-CREATOR.md): a save is checked as a catalog entry (422 invalid_catalog { problems }); publish writes
 * the entry into the catalog table (physical changes: Reviews' `physical` rule); unpublish is refused (422
 * not_supported): an entry is removed with a tombstone version (`removed: true`) – publishing it takes it out of the
 * catalog (409 in_use while used) – or, never published, deleted (batch `deletes`).
 */
final class EntitiesController
{
    public function __construct(private readonly EntityStore $store, private readonly Roles $roles, private readonly AssetUsages $usages, private readonly Reviews $reviews, private readonly ContentSchemas $schemas, private readonly FlowCheck $flow, private readonly ItemPlacements $placements, private readonly CatalogDocs $docs, private readonly MapRefs $mapRefs, private readonly Scenes $scenes, private readonly PublishDependencies $dependencies, private readonly IdleCheck $idles)
    {
    }

    public function index(Request $request, Response $response): Response
    {
        $this->creator($request);
        $query = $request->getQueryParams();
        if (is_string($query['kinds'] ?? null)) {
            $kinds = array_values(array_filter(array_map('trim', explode(',', $query['kinds']))));
            if (in_array('show', $kinds, true)) throw new ApiError(403, 'forbidden', 'A show goes through /api/shows.', ['role' => Roles::LIVE]);
            if (in_array('scene', $kinds, true)) throw self::sceneRoute();
            return Json::response($response, 200, ['status' => 'success', 'entities' => $this->store->withContent($kinds, ($query['content'] ?? 'draft') !== 'published')]);
        }
        $index = $this->store->index(array_key_exists('toon', $query) ? $query['toon'] : null);
        // a factory module is read-only and built in the catalog: an old draft of it is never something to publish
        $factory = array_flip($this->docs->factoryModuleKeys());
        if ($factory !== []) $index['entities'] = array_map(fn(array $e) => isset($factory[$e['key']]) ? $e + ['read_only' => true] : $e, $index['entities']);
        return Json::response($response, 200, $index);
    }

    /** @param array{key: string} $args */
    public function get(Request $request, Response $response, array $args): Response
    {
        $key = (string) EntityKey::parse($args['key']);
        $this->creator($request, $args['key']);
        if (NewestVersion::asked($request)) return NewestVersion::answer($request, $response, $this->store, $key);
        $query = $request->getQueryParams();
        $row = $this->store->read($key, ($query['draft'] ?? '') === '1', isset($query['version']) ? (int) $query['version'] : 0);
        if (($query['wrap'] ?? '') === '1') return Json::response($response, 200, $this->store->wrapped($row));
        $response->getBody()->write((string) $row['json']);
        return $response->withStatus(200)
            ->withHeader('Content-Type', 'application/json; charset=utf-8')
            ->withHeader('Cache-Control', 'no-store')
            ->withHeader('X-Content-Version', (string) $row['id'])
            ->withHeader('X-Content-Status', (string) $row['status'])
            ->withHeader('Access-Control-Expose-Headers', 'X-Content-Version, X-Content-Status');
    }

    /** @param array{key: string} $args */
    public function put(Request $request, Response $response, array $args): Response
    {
        $key = EntityKey::parse($args['key']);
        $user = $this->creator($request, $args['key']);
        $body = $this->body($request);
        if (property_exists($body, 'content')) $this->guardContent($key, $body->content);
        // an AI connection signs its saves with its name (the History shows who – and through what – saved)
        $ai = CurrentSession::of($request)->apiToken();
        if ($ai !== null) $body->message = mb_substr(trim((is_string($body->message ?? null) ? $body->message : '') . " (AI: {$ai['name']})"), 0, 255);
        $result = $this->store->save($user, $key, $body);
        $this->usages->reindexEntity((string) $key); // the asset usage index (agents/ASSETS.md); never fails the save
        return Json::response($response, 200, $result);
    }

    /**
     * POST /api/entities/<key>/revert { version, content, base } · /revert-to { version, base } (#687): a revert is a new
     * draft, never a rewrite; `revert`'s content is checked like a save
     * @param array{key: string, action: string} $args
     */
    public function revert(Request $request, Response $response, array $args): Response
    {
        $key = EntityKey::parse($args['key']);
        $user = $this->creator($request, $args['key']);
        $body = $this->body($request);
        if ($key->isCatalog()) $this->docs->guardFactory($key);
        if ($args['action'] === 'revert' && property_exists($body, 'content')) $this->guardContent($key, $body->content);
        $result = $args['action'] === 'revert' ? $this->store->revert($user, $key, $body) : $this->store->revertTo($user, $key, $body);
        $this->usages->reindexEntity((string) $key);
        return Json::response($response, 200, $result);
    }

    /** a document is checked before it is stored: its type, the reader's flow, item limits, the map's references */
    private function guardContent(EntityKey $key, mixed $content): void
    {
        if ($key->isCatalog()) {
            $this->docs->guardSave($key, $content);
            return;
        }
        $this->schemas->guard($key, $content);
        // where a reader could get stuck (a question whose choices can all be hidden, a divert / jump into nothing)
        $this->flow->guard($key, $content);
        // an item placed more often than its catalog limit allows, counted over every toon (BACKLOG 10.39)
        $this->placements->guard($key, $content);
        // a route through a place that is not on the map / a place taken off a route's way (owner D7)
        $this->mapRefs->guard([(string) $key => $content]);
        // a character's idle that doesn't play on its model (#817, the core's FitCheck)
        $this->idles->guard($key, $content);
    }

    /** POST /api/entities/batch – several saves in one transaction (each document checked like a PUT) */
    public function batch(Request $request, Response $response): Response
    {
        $user = $this->creator($request);
        $body = $this->body($request);
        $list = is_array($body->saves ?? null) ? $body->saves : [];
        $deletes = array_values(array_map(fn(mixed $k) => EntityKey::parse($k), is_array($body->deletes ?? null) ? $body->deletes : []));
        if (($list === [] && $deletes === []) || count($list) + count($deletes) > 500) throw new ApiError(400, 'invalid_saves', 'saves / deletes must hold 1–500 documents.');
        $ai = CurrentSession::of($request)->apiToken();
        $saves = [];
        foreach ($list as $save) {
            if (!$save instanceof stdClass || !property_exists($save, 'content')) throw new ApiError(400, 'invalid_saves', 'every save needs { key, content }.');
            $key = EntityKey::parse($save->key ?? null);
            if ($key->kind === 'show') throw new ApiError(403, 'forbidden', 'A show goes through /api/shows.', ['role' => Roles::LIVE]);
            if ($key->kind === 'scene') throw self::sceneRoute();
            try {
                if ($key->isCatalog()) {
                    $this->docs->guardSave($key, $save->content);
                } else {
                    $this->schemas->guard($key, $save->content);
                    $this->flow->guard($key, $save->content);
                    $this->placements->guard($key, $save->content);
                    $this->idles->guard($key, $save->content);
                }
            } catch (ApiError $e) {
                throw new ApiError($e->status, $e->errorCode, $e->getMessage(), ['key' => (string) $key] + $e->extra);
            }
            $message = is_string($save->message ?? null) && trim($save->message) !== '' ? $save->message : (is_string($body->message ?? null) ? $body->message : null);
            if ($ai !== null) $message = mb_substr(trim((string) $message . " (AI: {$ai['name']})"), 0, 255);
            $one = new stdClass();
            $one->content = $save->content;
            $one->message = $message;
            if (property_exists($save, 'base')) $one->base = $save->base;
            $saves[] = ['key' => $key, 'body' => $one];
        }
        foreach ($deletes as $key) {
            if ($key->isCatalog()) $this->docs->guardDelete($key);
        }
        // the map's references judged on the whole batch (a new place and the route through it together are fine; owner D7)
        $this->mapRefs->guard(array_combine(array_map(fn(array $s) => (string) $s['key'], $saves), array_map(fn(array $s) => $s['body']->content, $saves)) ?: [], array_map('strval', $deletes));
        $result = $this->store->saveMany($user, $saves, $deletes);
        foreach ([...array_column($saves, 'key'), ...$deletes] as $key) $this->usages->reindexEntity((string) $key);
        return Json::response($response, 200, $result);
    }

    /** POST /api/entities/publish – several documents live at once (the review rules per key, like a single publish) */
    public function publishMany(Request $request, Response $response): Response
    {
        $user = $this->creator($request);
        $body = $this->body($request);
        $list = is_array($body->versions ?? null) ? $body->versions : throw new ApiError(400, 'missing_versions', 'versions is required.');
        if ($list === [] || count($list) > 500) throw new ApiError(400, 'invalid_versions', 'versions must hold 1–500 documents.');
        if (($body->partial ?? false) === true) return Json::response($response, 200, $this->publishPartial($user, $list, ($body->with_dependencies ?? false) === true));
        $versions = [];
        $review = [];
        $catalog = false;
        foreach ($list as $v) {
            $key = EntityKey::parse($v instanceof stdClass ? ($v->key ?? null) : null);
            if ($key->kind === 'scene') throw self::sceneRoute();
            $version = $v instanceof stdClass && isset($v->version) ? (int) $v->version : null;
            // every key is checked (the catalog's `physical` rule included) – the ones that need a review are named together
            try {
                $this->reviews->guardPublish($user, $key, $version);
            } catch (ApiError $e) {
                if ($e->errorCode !== 'review_required') throw $e;
                $review[] = (string) $key;
            }
            $catalog = $catalog || $key->isCatalog();
            $versions[] = ['key' => (string) $key, 'version' => $version];
        }
        // all or nothing: the caller submits these for review and publishes the rest
        if ($review !== []) throw new ApiError(403, 'review_required', 'Some of these changes need a review – submit them for review instead.', ['keys' => $review]);
        // a catalog entry's version writes the catalog table too (CatalogDocs), in the same transaction
        $by = (string) ($user['email'] ?? $user['id'] ?? 'creator');
        $result = $this->store->publishMany($versions, function (array $row) use ($by): bool {
            if (!EntityKey::parse((string) $row['entity_key'])->isCatalog()) return false;
            try {
                $this->docs->publishIn($row, $by);
            } catch (ApiError $e) {
                // an entry the catalog refuses is named by its key: the caller leaves it out and publishes the rest
                if ($e->errorCode !== 'invalid_catalog') throw $e;
                throw new ApiError($e->status, $e->errorCode, $e->getMessage(), ['key' => (string) $row['entity_key']] + $e->extra);
            }
            return true;
        });
        if ($catalog) $this->docs->reindex();
        foreach ($result['published'] as $p) {
            $this->reviews->markPublished($user, $p['key'], (int) $p['version']['id']);
            $this->usages->reindexEntity($p['key']);
        }
        return Json::response($response, 200, $result);
    }

    /**
     * POST /api/entities/publish/plan { versions: [{ key, version? }] } – what a Publish of these takes along: their
     * unpublished draft dependencies, transitively (PublishDependencies) → { status, entities: [index row + needed_by] }
     */
    public function publishPlan(Request $request, Response $response): Response
    {
        $this->creator($request);
        $body = $this->body($request);
        $list = is_array($body->versions ?? null) ? $body->versions : throw new ApiError(400, 'missing_versions', 'versions is required.');
        if ($list === [] || count($list) > 500) throw new ApiError(400, 'invalid_versions', 'versions must hold 1–500 documents.');
        $added = $this->dependencies->of(self::versionMap($list));
        $factory = array_flip($this->docs->factoryModuleKeys());
        $rows = array_map(fn(array $e) => $e + ['needed_by' => $added[$e['key']]] + (isset($factory[$e['key']]) ? ['read_only' => true] : []), $this->store->indexOf(array_keys($added)));
        return Json::response($response, 200, ['status' => 'success', 'entities' => $rows]);
    }

    /**
     * @param array<mixed> $list [{ key, version? }]
     * @return array<string, int|null>
     */
    private static function versionMap(array $list): array
    {
        $versions = [];
        foreach ($list as $v) {
            $key = EntityKey::parse($v instanceof stdClass ? ($v->key ?? null) : null);
            $versions[(string) $key] = $v instanceof stdClass && isset($v->version) ? (int) $v->version : null;
        }
        return $versions;
    }

    /**
     * `partial: true` (every Publish of the creator): in dependency order (PublishOrder), each unit on its own –
     * what can go live goes live; a refusal, a review or a skip is named with its reason, and what depends on it is skipped.
     * `with_dependencies: true` takes the unpublished draft dependencies along (PublishDependencies, their newest versions)
     * → { status, published: [{ key, version }], refused: [{ key, code, reason }], review: [key], added: [{ key, needed_by }] }
     * @param array<string, mixed> $user
     * @param array<mixed> $list
     * @return array<string, mixed>
     */
    private function publishPartial(array $user, array $list, bool $withDependencies = false): array
    {
        $added = [];
        if ($withDependencies) {
            foreach ($this->dependencies->of(self::versionMap($list)) as $key => $by) {
                $added[] = ['key' => $key, 'needed_by' => $by];
                $list[] = (object) ['key' => $key];
            }
        }
        $versions = [];
        /** @var array<string, int|null> $scenes */
        $scenes = [];
        $refused = [];
        $review = [];
        /** @var array<string, string> $failed key → what happened to it, for what depends on it */
        $failed = [];
        $refuse = function (string $key, string $code, string $reason, string $why) use (&$refused, &$failed): void {
            $refused[] = ['key' => $key, 'code' => $code, 'reason' => $reason];
            $failed[$key] = $why;
        };
        foreach ($list as $v) {
            $raw = $v instanceof stdClass ? ($v->key ?? null) : null;
            $key = EntityKey::parse($raw);
            $version = $v instanceof stdClass && isset($v->version) ? (int) $v->version : null;
            if ($key->kind === 'scene') {
                // a deck scene goes last, through its own check (the core's, as its page publishes it): after the modules,
                // instruments, presets and rigs it plays
                if (!$this->roles->canEditContent($user)) $refuse((string) $key, 'forbidden', 'Only content editors publish scenes.', 'was refused');
                else $scenes[(string) $key] = $version;
                continue;
            }
            try {
                $this->reviews->guardPublish($user, $key, $version);
            } catch (ApiError $e) {
                if ($e->errorCode !== 'review_required') {
                    $refuse((string) $key, $e->errorCode, $e->getMessage(), 'was refused');
                    continue;
                }
                $review[] = (string) $key;
                $failed[(string) $key] = 'waits for a review';
                continue;
            }
            $versions[(string) $key] = $version;
        }
        $rows = [];
        $refs = [];
        foreach ($versions as $key => $version) {
            $row = $version !== null ? $this->store->byId($version) : $this->store->latest($key);
            if (!$row || $row['entity_key'] !== $key) {
                $refuse($key, 'not_found', "Unknown version of $key.", 'was refused');
                continue;
            }
            $rows[$key] = $row;
            $parsed = EntityKey::parse($key);
            if ($parsed->isCatalog()) $refs[$key] = PublishOrder::refs($parsed, json_decode((string) $row['json'], true));
        }
        foreach ($scenes as $key => $version) {
            $row = $version !== null ? $this->store->byId($version) : $this->store->latest($key);
            if ($row && $row['entity_key'] === $key) $refs[$key] = PublishOrder::refs(EntityKey::parse($key), json_decode((string) $row['json'], true));
        }
        // why a key is skipped: the first thing it names that did not go live
        $needs = function (array $keys) use (&$failed, $refs): ?string {
            foreach ($keys as $key) {
                foreach ($refs[$key] ?? [] as $ref) {
                    if (isset($failed[$ref])) return 'needs ' . PublishOrder::label($ref) . ", which {$failed[$ref]}";
                }
            }
            return null;
        };
        $by = (string) ($user['email'] ?? $user['id'] ?? 'creator');
        $published = [];
        foreach (PublishOrder::units(array_keys($rows), $refs) as $unit) {
            $blocked = $needs($unit);
            if ($blocked !== null) {
                foreach ($unit as $key) $refuse($key, 'dependency', $blocked, 'was skipped');
                continue;
            }
            foreach ($unit as $key) {
                $blocker = $this->docs->publishBlocker($rows[$key]);
                if ($blocker !== null) $blocked ??= [$key, $blocker];
            }
            if ($blocked !== null) {
                [$which, $reason] = $blocked;
                foreach ($unit as $key) $refuse($key, $key === $which ? 'not_publishable' : 'dependency', $key === $which ? $reason : 'goes together with ' . PublishOrder::label($which) . ", which $reason", 'was refused');
                continue;
            }
            try {
                $result = $this->store->publishMany(array_map(fn(string $k) => ['key' => $k, 'version' => (int) $rows[$k]['id']], $unit), function (array $row) use ($by): bool {
                    if (!EntityKey::parse((string) $row['entity_key'])->isCatalog()) return false;
                    $this->docs->publishIn($row, $by);
                    return true;
                });
                $published = [...$published, ...$result['published']];
            } catch (ApiError $e) {
                $this->docs->forget();
                $problems = is_array($e->extra['problems'] ?? null) ? implode('; ', array_map('strval', $e->extra['problems'])) : $e->getMessage();
                foreach ($unit as $key) $refuse($key, $e->errorCode, $problems, 'was refused');
            }
        }
        if (array_filter($published, fn(array $p) => EntityKey::parse($p['key'])->isCatalog()) !== []) $this->docs->reindex();
        foreach ($scenes as $key => $version) {
            $blocked = $needs([$key]);
            if ($blocked !== null) {
                $refuse($key, 'dependency', $blocked, 'was skipped');
                continue;
            }
            try {
                $result = $this->scenes->publish(EntityKey::parse($key), $version);
                $published[] = ['key' => $key, 'version' => $result['version'] ?? $result];
            } catch (ApiError $e) {
                $problems = is_array($e->extra['problems'] ?? null) ? implode('; ', array_map('strval', $e->extra['problems'])) : $e->getMessage();
                $refuse($key, $e->errorCode, $problems, 'was refused');
            }
        }
        foreach ($published as $p) {
            $this->reviews->markPublished($user, $p['key'], (int) $p['version']['id']);
            $this->usages->reindexEntity($p['key']);
        }
        return ['status' => 'success', 'published' => $published, 'refused' => $refused, 'review' => $review, 'added' => $added];
    }

    /** @param array{id: string} $args */
    public function itemPlacements(Request $request, Response $response, array $args): Response
    {
        $this->creator($request);
        return Json::response($response, 200, $this->placements->report($args['id']));
    }

    /** @param array{kind: string} $args */
    public function catalogDocs(Request $request, Response $response, array $args): Response
    {
        $this->creator($request);
        return Json::response($response, 200, $this->docs->list($args['kind']));
    }

    /** @param array{key: string} $args */
    public function versions(Request $request, Response $response, array $args): Response
    {
        $key = (string) EntityKey::parse($args['key']);
        $this->creator($request, $args['key']);
        return Json::response($response, 200, $this->store->versions($key, (int) ($request->getQueryParams()['limit'] ?? 100)));
    }

    /** @param array{key: string, action: string} $args */
    public function action(Request $request, Response $response, array $args): Response
    {
        $key = (string) EntityKey::parse($args['key']);
        $user = $this->creator($request, $args['key']);
        $body = JsonBody::raw($request) === '' ? new stdClass() : $this->body($request);
        $version = isset($body->version) ? (int) $body->version : null;
        $parsed = EntityKey::parse($key);
        if ($parsed->isCatalog() && $args['action'] === 'unpublish') {
            throw new ApiError(422, 'not_supported', 'A catalog entry is not unpublished – the admin removes it from the catalog (its uses are checked there).');
        }
        if ($args['action'] !== 'restore') $this->reviews->guardPublish($user, EntityKey::parse($key), $version, $args['action'] === 'unpublish');
        elseif ($parsed->isCatalog()) $this->docs->guardFactory($parsed);
        $result = match ($args['action']) {
            'publish' => $parsed->isCatalog() ? $this->docs->publish($user, $parsed, $version) : $this->store->publish($key, $version),
            'unpublish' => $this->store->unpublish($key),
            'restore' => $this->store->restore($user, $key, $body),
            default => throw new ApiError(404, 'not_found', 'Unknown endpoint.'),
        };
        if ($args['action'] === 'publish') $this->reviews->markPublished($user, $key, (int) $result['version']['id']);
        $this->usages->reindexEntity($key);
        return Json::response($response, 200, $result);
    }

    /** @return array<string, mixed> */
    private function creator(Request $request, ?string $key = null): array
    {
        // #90: shows are the `live` role's (ShowsController); creator does not imply it
        if ($key !== null && str_starts_with($key, 'show:')) throw new ApiError(403, 'forbidden', 'A show goes through /api/shows.', ['role' => Roles::LIVE]);
        if ($key !== null && str_starts_with($key, 'scene:')) throw self::sceneRoute();
        $session = CurrentSession::of($request);
        $session->rejectCsrf();
        return $this->roles->requireContentEditor($session->user());
    }

    /**
     * #243: a scene's version is stamped and its publish checked by the core – only /api/scenes does both
     * (ScenesController); `scene:` = the deck scene, never a chapter group
     */
    private static function sceneRoute(): ApiError
    {
        return new ApiError(400, 'wrong_route', 'A scene goes through /api/scenes.');
    }

    /** The request body decoded with objects (so `{}` stays an object) */
    private function body(Request $request): stdClass
    {
        $raw = JsonBody::raw($request);
        if (strlen($raw) > $this->store->maxBytes() + 65536) {
            throw new ApiError(413, 'content_too_large', 'The document is larger than ' . $this->store->maxBytes() . ' bytes.');
        }
        return JsonBody::object($request) ?? throw new ApiError(400, 'invalid_json', 'The request body is not a JSON object.');
    }
}
