<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Catalog\CatalogStore;
use Scutoons\Domain\Rigs\KindCatalog;
use Scutoons\Domain\Modules\ModuleCompiler;
use Scutoons\Domain\Modules\ModuleSource;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;
use Scutoons\Support\Json as JsonEncoding;

/**
 * /api/modules – rig modules, Rust compiled to wasm (#281, Domain\Modules; agents/research/RIG-EDITING.md §4–7;
 * docs/API.md "Modules"). Modules are the factory's (catalog_modules, documents `module:<id>` in the creator); a session,
 * or a paired deck's token (a deck loads the .wasm it plays), reads; the compile is the creators'.
 *   GET    /kinds                → { status, catalog_version, kinds: [Descriptor] } – the node-kind catalog (deck/catalog, generated kinds.json: ports,
 *                                  parameters, docs) plus the published modules as `module:<id>` · a session or a deck token
 *   GET    /modules/{id}         → { status, module: { id, name?, files, descriptor?, wasm?: { hash, bytes, toolchain?, url } } } –
 *                                  a published factory module · 404 not_found
 *   POST   /modules/compile      { files?, factory?: "<id>" } → { status, hash, descriptor, wasm: { hash, bytes, url }, warnings, module? }
 *                                  creators only (403 creator_only); with `factory` the result goes into a new draft of
 *                                  `module:<id>` – a factory module (#350): 403 factory_module, except an admin's build
 *                                  (no files: its stored sources, the result into its catalog entry) ·
 *                                  422 compile_failed { errors: [{ file, line, column, level, message }] } ·
 *                                  503 live_show | build_busy | no_build_service | build_unavailable
 */
final class ModulesController
{
    public function __construct(
        private readonly ModuleCompiler $compiler,
        private readonly CatalogStore $catalog,
    ) {
    }

    public function kinds(Request $request, Response $response): Response
    {
        CurrentSession::of($request)->requireUser();
        return self::json($response, 200, ['status' => 'success'] + KindCatalog::merged($this->catalog->map('modules')));
    }

    /** @param array{id: string} $args */
    public function get(Request $request, Response $response, array $args): Response
    {
        CurrentSession::of($request)->requireUser();
        $module = $this->catalog->get('modules', $args['id']);
        if (!is_array($module)) throw new ApiError(404, 'not_found', "No module {$args['id']}.");
        return self::json($response, 200, ['status' => 'success', 'module' => $this->withUrl($module)]);
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
        $factory = is_string($body->factory ?? null) ? $body->factory : null;
        if ($files === null && $factory === null) throw new ApiError(400, 'missing_files', 'Send files, or name the factory module to compile.');
        $result = $this->compiler->compile($user, $files, $factory);
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
