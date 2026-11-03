<?php
declare(strict_types=1);

namespace Scutoons\Tests\Http;

use Psr\Http\Message\ResponseInterface;
use RuntimeException;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Domain\Catalog\CatalogDocs;
use Scutoons\Domain\Content\EntityKey;
use Scutoons\Domain\Modules\ModuleSource;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Http\DeckTokenScope;
use Scutoons\Infrastructure\AssetStorage\HttpClient;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Kernel;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

/** The build service as the tests see it: records what the API sent, answers what the test says */
final class FakeBuildService implements HttpClient
{
    /** @var list<array{method: string, url: string, headers: array<string, string>, body: ?string}> */
    public array $requests = [];
    /** @var array{status: int, body: string, headers: array<string, string>}|RuntimeException */
    public array|RuntimeException $answer;

    public function __construct()
    {
        $this->answer = ['status' => 503, 'body' => '{"error":"busy"}', 'headers' => []];
    }

    public function send(string $method, string $url, array $headers = [], ?string $body = null, ?string $bodyFile = null, ?string $sinkFile = null): array
    {
        $this->requests[] = ['method' => $method, 'url' => $url, 'headers' => $headers, 'body' => $body];
        if ($this->answer instanceof RuntimeException) throw $this->answer;
        return $this->answer;
    }
}

/**
 * /api/modules (#281, agents/research/RIG-EDITING.md §4–7): published modules by id; the compile for creators only,
 * through the build service (a fake here), the .wasm stored on the CDN by its hash, the descriptor + hash written into
 * a draft of the factory document `module:<id>`; the catalog kind `modules`.
 */
final class ModulesApiTest extends TestCase
{
    private const WASM = "\0asm\x01\0\0\0";
    private const SOURCE = ['lib.rs' => 'use deck_module::Module; #[derive(Module)] struct M;'];
    private const DESCRIPTOR = ['abi' => 1, 'name' => 'm', 'version' => '0.1.0',
        'params' => [['name' => 'spread', 'min' => 0.0, 'max' => 1.0, 'default' => 0.0]], 'ports' => [['name' => 'cv', 'dir' => 'out', 'kind' => 'audio']]];

    private FakeBuildService $build;

    protected function settings(): array
    {
        return [
            'ASSET_STORAGE' => 'local',
            'ASSET_LOCAL_ROOT' => $this->dir . '/cdn',
            'CDN_URL' => 'http://cdn.test/',
            'MODULE_BUILD_URL' => 'http://build.test:7710/',
            'MODULE_BUILD_TOKEN' => 'shared-secret',
        ] + parent::settings();
    }

    protected function setUp(): void
    {
        parent::setUp();
        $this->build = new FakeBuildService();
    }

    /** @param array<string, string> $headers */
    private function request(string $method, string $uri, array $headers = [], mixed $body = null): ResponseInterface
    {
        $request = (new ServerRequestFactory())->createServerRequest($method, "http://localhost$uri", ['REMOTE_ADDR' => '127.0.0.1']);
        foreach ($headers as $name => $value) $request = $request->withHeader($name, $value);
        if ($body !== null) {
            $request->getBody()->write((string) json_encode($body, JSON_PRESERVE_ZERO_FRACTION));
            $request->getBody()->rewind();
            $request = $request->withHeader('Content-Type', 'application/json');
        }
        $container = Kernel::container(Settings::fixed(self::root(), $this->settings()));
        $container->set(Clock::class, $this->clock);
        $container->set(HttpClient::class, $this->build);
        return Kernel::app($container)->handle($request);
    }

    /** @return array<string, mixed> */
    private static function json(ResponseInterface $response): array
    {
        return (array) json_decode((string) $response->getBody(), true);
    }

    /** @return array<string, string> the session header of a member (a creator with $creator) */
    private function as(string $email, bool $creator = false): array
    {
        $user = $this->user($email);
        if ($creator) $this->get(Roles::class)->grant((string) $user['id'], Roles::CREATOR);
        return ['Authorization' => 'Bearer ' . $this->get(Sessions::class)->create((string) $user['id'])];
    }

    /** the service's good answer for $wasm */
    private function builds(string $wasm = self::WASM): string
    {
        $hash = hash('sha256', $wasm);
        $this->build->answer = ['status' => 200, 'headers' => [], 'body' => (string) json_encode(['hash' => $hash, 'descriptor' => self::DESCRIPTOR,
            'wasm_b64' => base64_encode($wasm), 'toolchain' => 'rustc 1.99.1', 'warnings' => [['file' => 'lib.rs', 'line' => 1, 'column' => 5, 'level' => 'warning', 'message' => 'unused']]],
            JSON_PRESERVE_ZERO_FRACTION)];
        return $hash;
    }

    public function testTheKindCatalogListsTheBuiltInsAndThePublishedModules(): void
    {
        $me = $this->as('writer@scutoons.test');
        self::assertSame(401, $this->request('GET', '/api/kinds')->getStatusCode(), 'signed in only');
        $catalog = self::json($this->request('GET', '/api/kinds', $me));
        self::assertSame(1, $catalog['catalog_version']);
        $kinds = array_column($catalog['kinds'], null, 'name');
        // every kind the validator knows is in the catalog with the same input ports
        foreach (\Scutoons\Domain\Rigs\NodeKinds::KINDS as $name => $kind) {
            self::assertArrayHasKey($name, $kinds, $name);
            $in = array_column(array_filter($kinds[$name]['ports'], fn($p) => $p['dir'] === 'in'), 'type', 'name');
            self::assertSame($kind['inputs'], $in, "$name inputs");
        }
        $fm = $kinds['fm'];
        self::assertSame(['audio', 'synth', 'core'], [$fm['domain'], $fm['module'], $fm['feature']]);
        $ratio = array_values(array_filter($fm['params'], fn($p) => $p['name'] === 'ratio'))[0];
        self::assertSame(['log', 0.25, 8.0, 2.0, 'ratio'], [$ratio['curve'], $ratio['min'], $ratio['max'], $ratio['default'], $ratio['unit']]);
        self::assertNotSame('', $ratio['doc']);
        self::assertArrayNotHasKey('module:wobble', $kinds);

        // a published module joins as module:<id>; one without a compile does not
        $store = $this->get(\Scutoons\Domain\Catalog\CatalogStore::class);
        $store->save('modules', 'wobble', ['id' => 'wobble', 'files' => self::SOURCE, 'descriptor' => self::DESCRIPTOR], 'test');
        $store->save('modules', 'bare', ['id' => 'bare', 'files' => self::SOURCE], 'test');
        $store->forget();
        $kinds = array_column(self::json($this->request('GET', '/api/kinds', $me))['kinds'], null, 'name');
        self::assertArrayHasKey('module:wobble', $kinds);
        self::assertArrayNotHasKey('module:bare', $kinds);
        self::assertSame([['cv', 'out', 'audio']], array_map(fn($p) => [$p['name'], $p['dir'], $p['type']], $kinds['module:wobble']['ports']));
        self::assertSame(['spread', 0.0, 1.0], [$kinds['module:wobble']['params'][0]['name'], $kinds['module:wobble']['params'][0]['min'], $kinds['module:wobble']['params'][0]['max']]);
        self::assertSame(1, $kinds['module:wobble']['abi']);
    }

    public function testSourcesAreHeldToTheLimits(): void
    {
        $this->builds();
        $creator = $this->as('boss@scutoons.test', true);
        $refused = function (array $body, string $part) use ($creator): void {
            $r = $this->request('POST', '/api/modules/compile', $creator, $body);
            self::assertSame([422, 'invalid_module'], [$r->getStatusCode(), self::json($r)['error']], (string) json_encode($body));
            self::assertStringContainsString($part, (string) $r->getBody());
        };
        $refused(['files' => ['osc.rs' => '//']], 'needs a lib.rs');
        $refused(['files' => ['lib.rs' => '//', 'Osc.rs' => '//']], 'a name is [a-z0-9_]+.rs');
        $refused(['files' => ['lib.rs' => '//', 'build.rs' => '//', 'a/b.rs' => '//']], 'a name is [a-z0-9_]+.rs');
        $refused(['files' => ['lib.rs' => str_repeat('a', ModuleSource::MAX_FILE_BYTES + 1)]], 'larger than');
        $refused(['files' => ['lib.rs' => str_repeat('a', 200000), 'a.rs' => str_repeat('a', 200000), 'b.rs' => str_repeat('a', 200000)]], 'together are larger');
        $refused(['files' => array_combine(array_map(fn(int $i) => "f$i.rs", range(0, 32)), array_fill(0, 33, '//'))], '1–32 files');
        $refused(['files' => ['lib.rs' => 5]], 'must be a text');
        self::assertSame([], $this->build->requests, 'the service was not asked');
    }

    public function testOnlyCreatorsCompile(): void
    {
        $this->builds();
        $r = $this->request('POST', '/api/modules/compile', $this->as('member@scutoons.test'), ['files' => self::SOURCE]);
        self::assertSame([403, 'creator_only'], [$r->getStatusCode(), self::json($r)['error']]);
        self::assertSame([], $this->build->requests, 'the service was not asked');
        self::assertSame(401, $this->request('POST', '/api/modules/compile', [], ['files' => self::SOURCE])->getStatusCode());
    }

    public function testADeckMayReadButNotWriteModules(): void
    {
        DeckTokenScope::guard('GET', '/api/modules/walk');
        foreach ([['PUT', '/api/modules/walk'], ['DELETE', '/api/modules/walk'], ['POST', '/api/modules/compile']] as [$method, $path]) {
            try {
                DeckTokenScope::guard($method, $path);
                self::fail("$method $path");
            } catch (ApiError $e) {
                self::assertSame('not_for_deck', $e->errorCode);
            }
        }
    }

    public function testWithoutABuildServiceNothingCompiles(): void
    {
        $creator = $this->as('boss@scutoons.test', true);
        $this->builds();
        foreach ([['MODULE_BUILD_URL' => ''], ['MODULE_BUILD_TOKEN' => '']] as $off) {
            $request = (new ServerRequestFactory())->createServerRequest('POST', 'http://localhost/api/modules/compile', ['REMOTE_ADDR' => '127.0.0.1']);
            $request->getBody()->write((string) json_encode(['files' => self::SOURCE]));
            $request->getBody()->rewind();
            $container = Kernel::container(Settings::fixed(self::root(), $off + $this->settings()));
            $container->set(Clock::class, $this->clock);
            $container->set(HttpClient::class, $this->build);
            $r = Kernel::app($container)->handle($request->withHeader('Content-Type', 'application/json')->withHeader('Authorization', $creator['Authorization']));
            self::assertSame([503, 'no_build_service'], [$r->getStatusCode(), self::json($r)['error']]);
        }
        self::assertSame([], $this->build->requests);
    }

    public function testACompileStoresTheWasmByItsHash(): void
    {
        $hash = $this->builds();
        $creator = $this->as('boss@scutoons.test', true);

        $r = $this->request('POST', '/api/modules/compile', $creator, ['files' => self::SOURCE]);
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
        $answer = self::json($r);
        self::assertSame([$hash, self::DESCRIPTOR], [$answer['hash'], $answer['descriptor']]);
        self::assertArrayNotHasKey('module', $answer, 'sources alone are written nowhere');
        self::assertSame(['hash' => $hash, 'bytes' => strlen(self::WASM), 'toolchain' => 'rustc 1.99.1', 'url' => "http://cdn.test/modules/$hash.wasm"], $answer['wasm']);
        self::assertSame('unused', $answer['warnings'][0]['message']);

        // what the service was sent: the token, the sources, the right address
        $sent = $this->build->requests[0];
        self::assertSame(['POST', 'http://build.test:7710/compile', 'shared-secret'], [$sent['method'], $sent['url'], $sent['headers']['X-Build-Token']]);
        self::assertSame(['files' => self::SOURCE], json_decode((string) $sent['body'], true));
        // the file is on the CDN under its hash, and is not written again
        $file = $this->dir . "/cdn/modules/$hash.wasm";
        self::assertSame(self::WASM, file_get_contents($file));
        touch($file, 1000000000);
        $this->request('POST', '/api/modules/compile', $creator, ['files' => self::SOURCE]);
        self::assertSame(1000000000, filemtime($file), 'content-addressed: same hash, same file');
    }

    public function testACompileOfAFactoryModuleMakesADraftThatPublishesIntoTheCatalog(): void
    {
        $hash = $this->builds();
        $creator = $this->as('boss@scutoons.test', true);
        $files = ['lib.rs' => '// walk', 'osc.rs' => '// osc'];
        $r = $this->request('POST', '/api/modules/compile', $creator, ['factory' => 'walk', 'files' => $files]);
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
        self::assertSame('module:walk', self::json($r)['module']);
        $key = EntityKey::parse('module:walk');
        $draft = json_decode((string) $this->db->value('SELECT json FROM entity_versions WHERE entity_key = ? ORDER BY id DESC LIMIT 1', ['module:walk']), true);
        self::assertSame(['walk', $files, self::DESCRIPTOR, $hash], [$draft['id'], $draft['files'], $draft['descriptor'], $draft['wasm']['hash']]);
        self::assertNull($this->get(\Scutoons\Domain\Catalog\CatalogStore::class)->get('modules', 'walk'), 'a draft is not the catalog');
        self::assertSame(404, $this->request('GET', '/api/modules/walk', $creator)->getStatusCode(), 'readers see published modules only');

        // the compile of the draft without files uses the draft's sources
        $asked = count($this->build->requests);
        $this->request('POST', '/api/modules/compile', $creator, ['factory' => 'walk']);
        self::assertSame(['files' => $files], json_decode((string) $this->build->requests[$asked]['body'], true));
        // an unknown factory module needs files
        self::assertSame(404, $this->request('POST', '/api/modules/compile', $creator, ['factory' => 'nothing'])->getStatusCode());
        self::assertSame(400, $this->request('POST', '/api/modules/compile', $creator, ['factory' => 'Bad Id', 'files' => $files])->getStatusCode());

        // publishing writes the catalog; every member then reads sources, descriptor and the wasm's address
        $user = $this->db->one('SELECT * FROM users WHERE email = ?', ['boss@scutoons.test']);
        $this->get(CatalogDocs::class)->publish((array) $user, $key, null);
        $member = $this->as('listener@scutoons.test');
        $module = self::json($this->request('GET', '/api/modules/walk', $member))['module'];
        self::assertSame(['walk', $files, "http://cdn.test/modules/$hash.wasm"], [$module['id'], $module['files'], $module['wasm']['url']]);
        // the admin reads the kind like the instruments, and writes answer 409 edited_in_creator
        $admin = $this->as('admin@scutoons.test');
        $this->get(Roles::class)->grant((string) $this->db->value('SELECT id FROM users WHERE email = ?', ['admin@scutoons.test']), Roles::ADMIN);
        self::assertSame(['walk'], array_column(self::json($this->request('GET', '/api/admin/catalog/modules', $admin))['items'], 'id'));
        $put = $this->request('PUT', '/api/admin/catalog/modules/walk', $admin, ['data' => ['id' => 'walk', 'files' => $files]]);
        self::assertSame([409, 'edited_in_creator'], [$put->getStatusCode(), self::json($put)['error'] ?? null]);
    }

    public function testTheServicesRefusalsComeBackAsTheyAre(): void
    {
        $creator = $this->as('boss@scutoons.test', true);
        $compile = fn() => $this->request('POST', '/api/modules/compile', $creator, ['files' => self::SOURCE]);

        $errors = [['file' => 'lib.rs', 'line' => 3, 'column' => 9, 'level' => 'error', 'message' => 'cannot find value `x`']];
        $this->build->answer = ['status' => 422, 'headers' => [], 'body' => (string) json_encode(['errors' => $errors])];
        $r = $compile();
        self::assertSame([422, 'compile_failed', $errors], [$r->getStatusCode(), self::json($r)['error'], self::json($r)['errors']]);

        foreach ([['live_show', 'live_show'], ['busy', 'build_busy'], ['x', 'build_unavailable']] as [$why, $code]) {
            $this->build->answer = ['status' => 503, 'headers' => [], 'body' => (string) json_encode(['error' => $why])];
            $r = $compile();
            self::assertSame([503, $code], [$r->getStatusCode(), self::json($r)['error']], $why);
        }
        $this->build->answer = new RuntimeException('connection refused');
        self::assertSame([503, 'build_unavailable'], [$compile()->getStatusCode(), self::json($compile())['error']]);
        $this->build->answer = ['status' => 401, 'headers' => [], 'body' => '{"error":"unauthorized"}'];
        self::assertSame([502, 'build_failed'], [$compile()->getStatusCode(), self::json($compile())['error']], 'a refused token is our trouble, not the creator\'s');

        // the service's word is not taken: a hash that is not the file's, bytes that are no wasm
        $hash = $this->builds();
        $this->build->answer = ['status' => 200, 'headers' => [], 'body' => (string) json_encode(['hash' => str_repeat('a', 64), 'descriptor' => self::DESCRIPTOR, 'wasm_b64' => base64_encode(self::WASM)])];
        $r = $compile();
        self::assertSame(502, $r->getStatusCode());
        $this->build->answer = ['status' => 200, 'headers' => [], 'body' => (string) json_encode(['hash' => hash('sha256', 'GIF89a'), 'descriptor' => self::DESCRIPTOR, 'wasm_b64' => base64_encode('GIF89a')])];
        $r = $compile();
        self::assertSame(502, $r->getStatusCode());
        $this->build->answer = ['status' => 200, 'headers' => [], 'body' => (string) json_encode(['hash' => $hash, 'descriptor' => ['abi' => 'one'], 'wasm_b64' => base64_encode(self::WASM)])];
        $r = $compile();
        self::assertSame(502, $r->getStatusCode());
        self::assertFileDoesNotExist($this->dir . "/cdn/modules/$hash.wasm", 'nothing refused reaches the CDN');
    }
}
