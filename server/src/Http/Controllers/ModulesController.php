<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Catalog\CatalogStore;
use Scutoons\Domain\Modules\MemberModules;
use Scutoons\Domain\Modules\ModuleCompiler;
use Scutoons\Domain\Modules\ModuleSource;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;
use Scutoons\Support\Json as JsonEncoding;

/**
 * /api/modules – rig modules, Rust compiled to wasm (#281, Domain\Modules; agents/research/RIG-EDITING.md §4–7;
 * docs/API.md "Modules"). A session, or a paired deck's token for the reads (a deck loads the .wasm it plays). Every
 * member row is the member's own; the compile is the creators'.
 *   GET    /modules              → { status, items: [{ id, name, descriptor?, wasm?, created_at, updated_at }] } – the member's own, no sources
 *   GET    /modules/{id}         → { status, module: { id, name?, files, descriptor?, wasm?: { hash, bytes, toolchain?, url } } } –
 *                                  the member's own (`m:<ulid>`) or a published factory module (`<id>`) · 404 not_found
 *   POST   /modules              { name, files } → 201 { status, module } · 422 invalid_module { problems } · 422 too_many_modules
 *   PUT    /modules/{m:id}       { name?, files } → { status, module } – other sources drop the old descriptor / wasm
 *   DELETE /modules/{m:id}       → { status }
 *   POST   /modules/compile      { files?, member?: "m:…", factory?: "<id>" } → { status, hash, descriptor, wasm: { hash, bytes, url }, warnings, module? }
 *                                  creators only (403 creator_only); the result goes into the member's row / a new draft of
 *                                  `module:<id>` · 422 compile_failed { errors: [{ file, line, column, level, message }] } ·
 *                                  503 live_show | build_busy | no_build_service | build_unavailable
 */
final class ModulesController
{
    public function __construct(
        private readonly MemberModules $members,
        private readonly ModuleCompiler $compiler,
        private readonly CatalogStore $catalog,
    ) {
    }

    public function list(Request $request, Response $response): Response
    {
        $user = CurrentSession::of($request)->requireUser();
        return self::json($response, 200, ['status' => 'success', 'items' => array_map($this->withUrl(...), $this->members->list((string) $user['id']))]);
    }

    /** @param array{id: string} $args */
    public function get(Request $request, Response $response, array $args): Response
    {
        $user = CurrentSession::of($request)->requireUser();
        $id = $args['id'];
        if (str_starts_with($id, 'm:')) {
            $module = $this->members->get((string) $user['id'], $id);
        } else {
            $module = $this->catalog->get('modules', $id);
            if (!is_array($module)) throw new ApiError(404, 'not_found', "No module $id.");
        }
        return self::json($response, 200, ['status' => 'success', 'module' => $this->withUrl($module)]);
    }

    public function create(Request $request, Response $response): Response
    {
        $user = CurrentSession::of($request)->requireUser();
        [$name, $files] = MemberModules::fromBody(self::body($request));
        $module = $this->members->save((string) $user['id'], null, $name, $files);
        return self::json($response, 201, ['status' => 'success', 'module' => $this->withUrl($module)]);
    }

    /** @param array{id: string} $args */
    public function save(Request $request, Response $response, array $args): Response
    {
        $user = CurrentSession::of($request)->requireUser();
        $current = $this->members->get((string) $user['id'], $args['id']);
        [$name, $files] = MemberModules::fromBody(self::body($request), (string) $current['name']);
        $module = $this->members->save((string) $user['id'], $args['id'], $name, $files);
        return self::json($response, 200, ['status' => 'success', 'module' => $this->withUrl($module)]);
    }

    /** @param array{id: string} $args */
    public function delete(Request $request, Response $response, array $args): Response
    {
        $user = CurrentSession::of($request)->requireUser();
        $this->members->delete((string) $user['id'], $args['id']);
        return self::json($response, 200, ['status' => 'success']);
    }

    public function compile(Request $request, Response $response): Response
    {
        $user = CurrentSession::of($request)->requireUser();
        $body = self::body($request);
        $files = null;
        if (property_exists($body, 'files')) {
            if (!$body->files instanceof \stdClass) throw new ApiError(422, 'invalid_module', 'files must be { "lib.rs": "source", … }.', ['problems' => ['files must be { "lib.rs": "source", … }']]);
            $files = get_object_vars($body->files);
        }
        $member = is_string($body->member ?? null) ? $body->member : null;
        $factory = is_string($body->factory ?? null) ? $body->factory : null;
        if ($files === null && $member === null && $factory === null) throw new ApiError(400, 'missing_files', 'Send files, or name the module (member / factory) to compile.');
        $result = $this->compiler->compile($user, $files, $member, $factory);
        $result['wasm']['url'] = (string) ($result['wasm']['url'] ?? '');
        return self::json($response, 200, ['status' => 'success'] + $result);
    }

    /** a descriptor's ranges keep their floats (`1.0` stays `1.0`, as in the serde forms of the rigs) */
    private static function json(Response $response, int $status, mixed $data): Response
    {
        return Json::response($response, $status, $data, [], JsonEncoding::FLAGS | JSON_PRESERVE_ZERO_FRACTION);
    }

    private static function body(Request $request): \stdClass
    {
        return JsonBody::object($request) ?? throw new ApiError(400, 'invalid_json', 'The request body is not a JSON object.');
    }

    /**
     * A module with the CDN URL of its .wasm in `wasm.url`
     * @param array<string, mixed> $module
     * @return array<string, mixed>
     */
    private function withUrl(array $module): array
    {
        $hash = is_array($module['wasm'] ?? null) ? ($module['wasm']['hash'] ?? null) : null;
        if (is_string($hash) && preg_match(ModuleSource::HASH_PATTERN, $hash)) {
            $module['wasm']['url'] = $this->compiler->wasmUrl(ModuleSource::wasmPath($hash));
        }
        return $module;
    }
}
