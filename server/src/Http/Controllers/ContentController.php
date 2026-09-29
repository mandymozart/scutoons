<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Content\ContentService;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;
use stdClass;

/**
 * Content store of the creator editor (docs/API.md "Content API"):
 *   GET  /api/content/<toon>/<locale>/<path>.json    public: the published version (raw JSON, ETag), else 404
 *        ?draft=1 | ?version=<id> [&wrap=1]          creator: the latest version / that version / wrapped with meta
 *   PUT  /api/content/<toon>/<locale>/<path>.json    creator: { content, message?, base? } → new draft version
 *   GET  /api/content/versions?toon=&locale=&path=   creator: version history (newest first)
 *   GET  /api/content/index?toon=                    creator: every stored file with latest + published ids
 *   POST /api/content/publish | unpublish | restore  creator
 * The role checks run inside (not as route middleware) to keep the old order: an invalid path answers 400/404 first.
 */
final class ContentController
{
    public function __construct(private readonly ContentService $content, private readonly Roles $roles)
    {
    }

    /** @param array{rest: string} $args */
    public function __invoke(Request $request, Response $response, array $args): Response
    {
        $method = $request->getMethod();
        $rest = $args['rest'];
        return match (true) {
            $method === 'GET' && $rest === 'versions' => $this->versions($request, $response),
            $method === 'GET' && $rest === 'index' => $this->index($request, $response),
            $method === 'POST' && $rest === 'publish' => $this->publish($request, $response),
            $method === 'POST' && $rest === 'unpublish' => $this->unpublish($request, $response),
            $method === 'POST' && $rest === 'restore' => $this->restore($request, $response),
            $method === 'GET' || $method === 'PUT' => $this->file($request, $response, $rest),
            default => throw new ApiError(404, 'not_found', 'Unknown endpoint.'),
        };
    }

    private function file(Request $request, Response $response, string $rest): Response
    {
        [$toon, $locale, $path] = ContentService::parseFile($rest);
        if ($request->getMethod() === 'PUT') {
            $user = $this->creator($request);
            return Json::response($response, 200, $this->content->save($user, $toon, $locale, $path, $this->objectBody($request)));
        }

        $query = $request->getQueryParams();
        $draft = ($query['draft'] ?? '') === '1';
        $versionId = isset($query['version']) ? (int) $query['version'] : 0;
        $wrap = ($query['wrap'] ?? '') === '1';

        if (!$draft && $versionId === 0 && !$wrap) {
            $row = $this->content->published($toon, $locale, $path) ?? throw new ApiError(404, 'not_found', 'Not published.');
            return $this->raw($request, $response, $row, true);
        }

        // Drafts, single versions and the editor's wrapped form need the creator role
        $this->creator($request);
        $row = $this->content->read($toon, $locale, $path, $draft, $versionId);
        if (!$wrap) return $this->raw($request, $response, $row, false);
        return Json::response($response, 200, $this->content->wrapped($row));
    }

    /**
     * The stored JSON as-is; public responses revalidate with an ETag (version id)
     * @param array<string, mixed> $row
     */
    private function raw(Request $request, Response $response, array $row, bool $public): Response
    {
        $etag = '"v' . $row['id'] . '"';
        $response = $response->withHeader('Content-Type', 'application/json; charset=utf-8')
            ->withHeader('Access-Control-Expose-Headers', 'ETag, X-Content-Version, X-Content-Status')
            ->withHeader('X-Content-Version', (string) $row['id'])
            ->withHeader('X-Content-Status', (string) $row['status']);
        if ($public) {
            $response = $response->withHeader('Cache-Control', 'no-cache')->withHeader('ETag', $etag);
            if (trim($request->getHeaderLine('If-None-Match')) === $etag) return $response->withStatus(304);
        } else {
            $response = $response->withHeader('Cache-Control', 'no-store');
        }
        $response->getBody()->write((string) $row['json']);
        return $response->withStatus(200);
    }

    private function versions(Request $request, Response $response): Response
    {
        $this->creator($request);
        return Json::response($response, 200, $this->content->versions($request->getQueryParams()));
    }

    private function index(Request $request, Response $response): Response
    {
        $this->creator($request);
        return Json::response($response, 200, $this->content->index($request->getQueryParams()['toon'] ?? null));
    }

    private function publish(Request $request, Response $response): Response
    {
        $this->creator($request);
        return Json::response($response, 200, $this->content->publish($this->objectBody($request)));
    }

    private function unpublish(Request $request, Response $response): Response
    {
        $this->creator($request);
        return Json::response($response, 200, $this->content->unpublish($this->objectBody($request)));
    }

    private function restore(Request $request, Response $response): Response
    {
        $user = $this->creator($request);
        return Json::response($response, 200, $this->content->restore($user, $this->objectBody($request)));
    }

    /** @return array<string, mixed> */
    private function creator(Request $request): array
    {
        return $this->roles->require(CurrentSession::of($request)->user(), Roles::CREATOR);
    }

    /** The request body decoded with objects (so `{}` stays an object) */
    private function objectBody(Request $request): stdClass
    {
        $raw = JsonBody::raw($request);
        if (strlen($raw) > $this->content->maxBytes() + 65536) {
            throw new ApiError(413, 'content_too_large', 'The file is larger than ' . $this->content->maxBytes() . ' bytes.');
        }
        return JsonBody::object($request) ?? throw new ApiError(400, 'invalid_json', 'The request body is not a JSON object.');
    }
}
