<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Assets\AssetUsages;
use Scutoons\Domain\Catalog\CatalogDocs;
use Scutoons\Domain\Content\ContentSchemas;
use Scutoons\Domain\Content\EntityKey;
use Scutoons\Domain\Content\EntityStore;
use Scutoons\Domain\Content\FlowCheck;
use Scutoons\Domain\Content\ItemPlacements;
use Scutoons\Domain\Content\Reviews;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;
use stdClass;

/**
 * The creator's write path on the content store, by entity key (docs/API.md "Entities"; role creator for everything –
 * readers use the library endpoints):
 *   GET  /api/entities?toon=                     every stored entity of a toon (`toon=` empty: platform-wide ones)
 *   GET  /api/entities?kinds=place,route,map&content=draft|published   every entity of these kinds WITH its document +
 *                                                `first` (creation order) – a page that edits many documents (the world map)
 *   POST /api/entities/batch { saves: [{ key, content, base?, message? }], deletes?: [key], message? }   several new
 *                                                draft versions in one transaction (all or none; 409 / 422 name the `key`);
 *                                                `deletes`: never-published place / route keys (409 published otherwise)
 *   POST /api/entities/publish { versions: [{ key, version? }] }   several documents live at once (one transaction)
 *   GET  /api/entities/<key>[?draft=1|?version=<id>][&wrap=1]   published / newest / one version (raw or wrapped)
 *   PUT  /api/entities/<key>                     { content, message?, base? } → new draft version (409 conflict;
 *                                                422 invalid_content { problems } when it doesn't fit its type: ContentSchemas,
 *                                                or can get the reader stuck: FlowCheck; 422 item_limit { problems }
 *                                                when it adds placements over a catalog item's `placements`)
 *   GET  /api/entities/<key>/versions?limit=     history, newest first
 *   POST /api/entities/<key>/publish { version? } · /unpublish · /restore { id, message? }
 *   GET  /api/catalog/items/<id>/placements      where a catalog item is placed in every toon + its limits and owners
 *                                                (ItemPlacements, BACKLOG 10.39) – 404 unknown_item
 *   GET  /api/catalog/docs/<kind>                every catalog entry of a kind as a document (CatalogDocs::list)
 * publish / unpublish: 403 review_required for accounts that need a review for the toon (Reviews; submit through
 * /api/reviews instead).
 *
 * Catalog documents (`item:<id>`, `recipe:<id>`, `album:<id>`, `product:<id>`, `edition:<id>` – CatalogDocs,
 * agents/CATALOG-IN-CREATOR.md): a save is checked as a catalog entry (422 invalid_catalog { problems }); publish writes
 * the entry into the catalog table (physical changes: Reviews' `physical` rule); unpublish is refused (422
 * not_supported – an entry leaves the catalog through the admin, where its uses are checked).
 */
final class EntitiesController
{
    public function __construct(private readonly EntityStore $store, private readonly Roles $roles, private readonly AssetUsages $usages, private readonly Reviews $reviews, private readonly ContentSchemas $schemas, private readonly FlowCheck $flow, private readonly ItemPlacements $placements, private readonly CatalogDocs $docs)
    {
    }

    public function index(Request $request, Response $response): Response
    {
        $this->creator($request);
        $query = $request->getQueryParams();
        if (is_string($query['kinds'] ?? null)) {
            $kinds = array_values(array_filter(array_map('trim', explode(',', $query['kinds']))));
            return Json::response($response, 200, ['status' => 'success', 'entities' => $this->store->withContent($kinds, ($query['content'] ?? 'draft') !== 'published')]);
        }
        return Json::response($response, 200, $this->store->index(array_key_exists('toon', $query) ? $query['toon'] : null));
    }

    /** @param array{key: string} $args */
    public function get(Request $request, Response $response, array $args): Response
    {
        $key = (string) EntityKey::parse($args['key']);
        $this->creator($request);
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
        $user = $this->creator($request);
        $body = $this->body($request);
        if (property_exists($body, 'content') && $key->isCatalog()) {
            $this->docs->guardSave($key, $body->content);
        } elseif (property_exists($body, 'content')) {
            $this->schemas->guard($key, $body->content);
            // where a reader could get stuck (a question whose choices can all be hidden, a divert / jump into nothing)
            $this->flow->guard($key, $body->content);
            // an item placed more often than its catalog limit allows, counted over every toon (BACKLOG 10.39)
            $this->placements->guard($key, $body->content);
        }
        // an AI connection signs its saves with its name (the History shows who – and through what – saved)
        $ai = CurrentSession::of($request)->apiToken();
        if ($ai !== null) $body->message = mb_substr(trim((is_string($body->message ?? null) ? $body->message : '') . " (AI: {$ai['name']})"), 0, 255);
        $result = $this->store->save($user, $key, $body);
        $this->usages->reindexEntity((string) $key); // the asset usage index (agents/ASSETS.md); never fails the save
        return Json::response($response, 200, $result);
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
            try {
                $this->schemas->guard($key, $save->content);
                $this->flow->guard($key, $save->content);
                $this->placements->guard($key, $save->content);
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
        $versions = [];
        $review = [];
        $catalog = false;
        foreach ($list as $v) {
            $key = EntityKey::parse($v instanceof stdClass ? ($v->key ?? null) : null);
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
            $this->docs->publishIn($row, $by);
            return true;
        });
        if ($catalog) $this->docs->reindex();
        foreach ($result['published'] as $p) {
            $this->reviews->markPublished($user, $p['key'], (int) $p['version']['id']);
            $this->usages->reindexEntity($p['key']);
        }
        return Json::response($response, 200, $result);
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
        $this->creator($request);
        return Json::response($response, 200, $this->store->versions($key, (int) ($request->getQueryParams()['limit'] ?? 100)));
    }

    /** @param array{key: string, action: string} $args */
    public function action(Request $request, Response $response, array $args): Response
    {
        $key = (string) EntityKey::parse($args['key']);
        $user = $this->creator($request);
        $body = JsonBody::raw($request) === '' ? new stdClass() : $this->body($request);
        $version = isset($body->version) ? (int) $body->version : null;
        $parsed = EntityKey::parse($key);
        if ($parsed->isCatalog() && $args['action'] === 'unpublish') {
            throw new ApiError(422, 'not_supported', 'A catalog entry is not unpublished – the admin removes it from the catalog (its uses are checked there).');
        }
        if ($args['action'] !== 'restore') $this->reviews->guardPublish($user, EntityKey::parse($key), $version, $args['action'] === 'unpublish');
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
    private function creator(Request $request): array
    {
        $session = CurrentSession::of($request);
        $session->rejectCsrf();
        return $this->roles->requireContentEditor($session->user());
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
