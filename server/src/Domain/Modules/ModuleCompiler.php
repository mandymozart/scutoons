<?php
declare(strict_types=1);

namespace Scutoons\Domain\Modules;

use RuntimeException;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Catalog\CatalogAdmin;
use Scutoons\Domain\Catalog\CatalogDocs;
use Scutoons\Domain\Catalog\CatalogStore;
use Scutoons\Domain\Content\EntityKey;
use Scutoons\Domain\Content\EntityStore;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Infrastructure\AssetStorage\AssetStorages;
use Scutoons\Infrastructure\AssetStorage\HttpClient;
use Scutoons\Infrastructure\AssetStorage\StorageKey;
use Scutoons\Infrastructure\Config\Settings;
use stdClass;
use Throwable;

/**
 * The compile of a rig module (#281, agents/research/RIG-EDITING.md §4, §7): the sources go to the build service
 * (deck/build, `MODULE_BUILD_URL`, the shared secret `MODULE_BUILD_TOKEN` as `X-Build-Token`) – a second container in the
 * relay's bunny app, rustc + the SDK, never running the code. Only creators compile (`creator_only`), because a compile
 * costs the relay's server CPU (owner 2026-11-02). The .wasm comes back with its SHA-256 and the descriptor read from it;
 * it is stored on the CDN as `modules/<hash>.wasm` (content-addressed, so immutable) and the descriptor + hash are written
 * into the module – a new draft of the factory document `module:<id>` (versions, review, publish as
 * any catalog document). Compile errors come back as `422 compile_failed { errors: [{ file, line, column, level, message }] }`.
 *
 * A factory module (#350, ModuleSource::isFactory) is read-only: a compile into it is refused (403 factory_module) – except
 * its own build: an admin compiles its stored sources unchanged (no `files`), and the descriptor + wasm go straight into
 * its catalog entry (CatalogAdmin, a published version; no draft – there is nothing to review, the sources did not change).
 * That is how the factory modules the migration imports get their wasm: the service compiles them once.
 */
final class ModuleCompiler
{
    /** the largest .wasm taken from the service (the service's own limit is lower) */
    private const MAX_WASM_BYTES = 4194304;

    public function __construct(
        private readonly Settings $settings,
        private readonly AssetStorages $storages,
        private readonly EntityStore $entities,
        private readonly CatalogDocs $docs,
        private readonly Roles $roles,
        private readonly HttpClient $http,
        private readonly CatalogStore $catalog,
        private readonly CatalogAdmin $admin,
    ) {
    }

    /**
     * Compiles and stores. Source: `$files`, or the factory module's own (`$factory`, a `module:<id>` document's newest
     * version). With `$factory` the result is written into a new draft of it (`$files` given too: they replace its sources).
     * @param array<string, mixed> $user
     * @param array<string, mixed>|null $files
     * @return array<string, mixed> { hash, descriptor, wasm: { hash, bytes, toolchain?, url }, warnings, errors?, module? }
     */
    public function compile(array $user, ?array $files, ?string $factory = null): array
    {
        if (!$this->roles->has($user, Roles::CREATOR)) {
            throw new ApiError(403, 'creator_only', 'Only creators compile modules.');
        }
        $url = rtrim($this->settings->get('MODULE_BUILD_URL'), '/');
        $token = $this->settings->get('MODULE_BUILD_TOKEN');
        if ($url === '' || $token === '') throw new ApiError(503, 'no_build_service', 'The module build service is not set up.');
        $key = null;
        $document = null;
        if ($factory !== null) {
            if (!preg_match(ModuleSource::ID_PATTERN, $factory)) throw new ApiError(400, 'invalid_target', "\"$factory\" is not a module id.");
            $key = EntityKey::parse("module:$factory");
            $entry = $this->catalog->get('modules', $factory);
            if (ModuleSource::isFactory(is_array($entry) ? $entry : null)) return $this->buildFactory($user, $factory, (array) $entry, $files, $url, $token);
            $latest = $this->entities->latest((string) $key);
            $document = $latest !== null ? json_decode((string) $latest['json'], true) : null;
            if ($files === null && !is_array($document)) throw new ApiError(404, 'not_found', "No module $factory – send its files to start one.");
            $files ??= (array) ($document['files'] ?? []);
        }
        $problems = ModuleSource::filesProblems($files);
        if ($problems) throw new ApiError(422, 'invalid_module', implode('; ', $problems), ['problems' => $problems]);

        $built = $this->build($url, $token, (array) $files);
        $path = ModuleSource::wasmPath($built['hash']);
        $this->storeWasm($path, $built['wasm']);
        $wasm = ['hash' => $built['hash'], 'bytes' => strlen($built['wasm'])];
        if ($built['toolchain'] !== null) $wasm['toolchain'] = $built['toolchain'];

        $result = [];
        if ($key !== null) {
            $content = is_array($document) ? $document : [];
            $content['id'] = (string) $key->id;
            $content['files'] = $files;
            $content['descriptor'] = $built['descriptor'];
            $content['wasm'] = $wasm;
            $this->docs->guardSave($key, $content);
            $body = new stdClass();
            $body->content = json_decode((string) json_encode($content, JSON_PRESERVE_ZERO_FRACTION));
            $body->message = 'compiled ' . substr($built['hash'], 0, 12);
            $saved = $this->entities->save($user, $key, $body);
            $result['module'] = (string) $key;
            $result['version'] = $saved['version'];
        }
        return ['hash' => $built['hash'], 'descriptor' => $built['descriptor'], 'wasm' => $wasm + ['url' => $this->wasmUrl($path)], 'warnings' => $built['warnings']] + $result;
    }

    /**
     * A factory module's own build (an admin, its stored sources unchanged): the result is written into the catalog entry
     * @param array<string, mixed> $user
     * @param array<string, mixed> $entry the catalog entry
     * @param array<string, mixed>|null $files
     * @return array<string, mixed>
     */
    private function buildFactory(array $user, string $id, array $entry, ?array $files, string $url, string $token): array
    {
        if ($files !== null || !$this->roles->has($user, Roles::ADMIN)) throw ModuleSource::readOnly($id);
        $built = $this->build($url, $token, (array) ($entry['files'] ?? []));
        $path = ModuleSource::wasmPath($built['hash']);
        $this->storeWasm($path, $built['wasm']);
        $wasm = ['hash' => $built['hash'], 'bytes' => strlen($built['wasm'])];
        if ($built['toolchain'] !== null) $wasm['toolchain'] = $built['toolchain'];
        $entry['descriptor'] = $built['descriptor'];
        $entry['wasm'] = $wasm;
        $this->admin->put('modules', $id, json_decode((string) json_encode($entry, JSON_PRESERVE_ZERO_FRACTION)), (string) ($user['email'] ?? $user['id'] ?? 'admin'), null, 'built ' . substr($built['hash'], 0, 12));
        $published = $this->entities->published("module:$id");
        return ['hash' => $built['hash'], 'descriptor' => $built['descriptor'], 'wasm' => $wasm + ['url' => $this->wasmUrl($path)], 'warnings' => $built['warnings'],
            'module' => "module:$id"] + ($published !== null ? ['version' => (int) $published['id']] : []);
    }

    /** The CDN URL of a stored module's .wasm (GET /api/modules/<id>) */
    public function wasmUrl(string $path): string
    {
        return $this->storages->primary()->publicUrl($path);
    }

    /**
     * One request to the build service
     * @param array<string, mixed> $files
     * @return array{hash: string, descriptor: array<string, mixed>, wasm: string, toolchain: ?string, warnings: list<array<string, mixed>>}
     */
    private function build(string $url, string $token, array $files): array
    {
        try {
            $response = $this->http->send('POST', "$url/compile", ['Content-Type' => 'application/json', 'X-Build-Token' => $token],
                (string) json_encode(['files' => (object) $files], JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE));
        } catch (RuntimeException) {
            throw new ApiError(503, 'build_unavailable', 'The module build service does not answer.');
        }
        $body = json_decode($response['body'], true);
        $body = is_array($body) ? $body : [];
        switch ($response['status']) {
            case 200:
                return $this->accept($body);
            case 422:
                $errors = self::diagnostics($body['errors'] ?? null);
                throw new ApiError(422, 'compile_failed', $errors[0]['message'] ?? 'The module does not compile.', ['errors' => $errors]);
            case 503:
                $why = (string) ($body['error'] ?? '');
                if ($why === 'live_show') throw new ApiError(503, 'live_show', 'A show is live – compiles wait until it is over.');
                if ($why === 'busy') throw new ApiError(503, 'build_busy', 'The build service is busy – try again in a moment.');
                throw new ApiError(503, 'build_unavailable', 'The module build service is not available.');
            default:
                throw new ApiError(502, 'build_failed', 'The module build service answered ' . $response['status'] . '.');
        }
    }

    /**
     * What a successful answer must be: the bytes are a wasm of at most 4 MB whose SHA-256 is the hash given, with a
     * descriptor of the shape ModuleSource knows – we do not take the service's word for it
     * @param array<string, mixed> $body
     * @return array{hash: string, descriptor: array<string, mixed>, wasm: string, toolchain: ?string, warnings: list<array<string, mixed>>}
     */
    private function accept(array $body): array
    {
        $bytes = is_string($body['wasm_b64'] ?? null) ? base64_decode($body['wasm_b64'], true) : false;
        $descriptor = $body['descriptor'] ?? null;
        $ok = is_string($bytes) && $bytes !== '' && strlen($bytes) <= self::MAX_WASM_BYTES && str_starts_with($bytes, "\0asm")
            && is_array($descriptor) && ModuleSource::descriptorProblems($descriptor) === [];
        if (!$ok) throw new ApiError(502, 'build_failed', 'The module build service answered with something that is not a module.');
        /** @var string $bytes */
        $hash = hash('sha256', $bytes);
        if (($body['hash'] ?? null) !== $hash) throw new ApiError(502, 'build_failed', 'The build service\'s hash does not match its module.');
        $toolchain = is_string($body['toolchain'] ?? null) ? substr($body['toolchain'], 0, 100) : null;
        return ['hash' => $hash, 'descriptor' => $descriptor, 'wasm' => $bytes, 'toolchain' => $toolchain, 'warnings' => self::diagnostics($body['warnings'] ?? null)];
    }

    /**
     * The compiler's messages as the service sent them, cut to the five fields
     * @return list<array{file: string, line: int, column: int, level: string, message: string}>
     */
    private static function diagnostics(mixed $list): array
    {
        $out = [];
        foreach (is_array($list) ? array_slice($list, 0, 100) : [] as $d) {
            if (!is_array($d)) continue;
            $out[] = ['file' => substr((string) ($d['file'] ?? ''), 0, 100), 'line' => (int) ($d['line'] ?? 0), 'column' => (int) ($d['column'] ?? 0),
                'level' => substr((string) ($d['level'] ?? 'error'), 0, 16), 'message' => substr((string) ($d['message'] ?? ''), 0, 2000)];
        }
        return $out;
    }

    /**
     * Stores the .wasm at `modules/<hash>.wasm` on the media storage – the platform's own folder, outside the asset
     * registry (StorageKey::SYSTEM_PREFIXES: the importer and the uploads leave it alone). Content-addressed: a file
     * that exists is the same bytes, so it is not written again.
     */
    private function storeWasm(string $path, string $bytes): void
    {
        $storage = $this->storages->primary();
        try {
            if ($storage->stat($path) !== null) return;
            $tmp = tempnam(sys_get_temp_dir(), 'wasm');
            if ($tmp === false) throw new RuntimeException('No temporary file.');
            try {
                file_put_contents($tmp, $bytes);
                $storage->upload($path, $tmp, StorageKey::mime($path));
            } finally {
                @unlink($tmp);
            }
        } catch (Throwable $e) {
            throw new ApiError(503, 'storage_failed', 'The compiled module could not be stored on the CDN.', ['detail' => $e->getMessage()]);
        }
    }
}
