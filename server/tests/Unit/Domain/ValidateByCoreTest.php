<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Psr\Http\Message\ResponseInterface;
use RuntimeException;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Domain\Catalog\CatalogAdmin;
use Scutoons\Domain\Catalog\CatalogStore;
use Scutoons\Domain\Catalog\CatalogValidator;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Infrastructure\AssetStorage\HttpClient;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Kernel;
use Scutoons\Tests\FakeCore;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;
use stdClass;

/**
 * #313: instruments, presets and rigs are validated by the Rust core – the API forwards the document with what it names
 * (by value, from its catalog) to the build service's `POST /validate` on every catalog write and returns the core's
 * problems unchanged; without the service nothing is written (503); drafts are saved without it. The fixture set
 * (fixtures/rigs/validate.json) holds the requests and the retired PHP RigValidator's words on them: the API builds those
 * requests from its catalog here, the core answers those words in deck/build's and deck-space's tests.
 */
final class ValidateByCoreTest extends TestCase
{
    private const BY = 'admin:boss@example.com';

    private FakeCore $core;
    private bool $withService = true;

    protected function settings(): array
    {
        return ($this->withService ? FakeCore::settings() : []) + parent::settings();
    }

    protected function setUp(): void
    {
        parent::setUp();
        $this->core = new FakeCore();
        $this->container->set(HttpClient::class, $this->core);
    }

    /** @return list<array<string, mixed>> the fixture set's cases, decoded with objects */
    private static function cases(): array
    {
        $set = json_decode((string) file_get_contents(self::root() . '/tests/fixtures/rigs/validate.json'));
        self::assertInstanceOf(stdClass::class, $set);
        $cases = [];
        foreach ((array) $set->cases as $c) {
            self::assertInstanceOf(stdClass::class, $c);
            $cases[] = get_object_vars($c);
        }
        return $cases;
    }

    /** a request compared as data: the lookups by id (their order is the API's) */
    private static function normal(mixed $request): mixed
    {
        $r = json_decode((string) json_encode($request), true);
        foreach (['instruments', 'presets'] as $list) {
            usort($r[$list], fn(array $a, array $b) => strcmp((string) $a['id'], (string) $b['id']));
        }
        return $r;
    }

    public function testTheApiSendsTheFixtureRequestsAndReturnsTheCoresWordsUnchanged(): void
    {
        $store = $this->get(CatalogStore::class);
        // what the fixture's choir cases name, in the catalog (written as they are – the table, not a check)
        $choir = null;
        foreach (self::cases() as $c) if ($c['kind'] === 'instruments' && $c['problems'] === []) $choir = $c['document'];
        self::assertInstanceOf(stdClass::class, $choir);
        $store->save('instruments', 'choir', $choir, self::BY);
        $store->save('presets', 'choir.base', json_decode('{"id":"choir.base","instrument":"choir","values":{"voice.ratio":1.5}}'), self::BY);
        $store->save('presets', 'choir.bright', json_decode('{"id":"choir.bright","instrument":"choir","parent":"choir.base"}'), self::BY);

        $validator = $this->get(CatalogValidator::class);
        $cases = self::cases();
        self::assertGreaterThanOrEqual(5, count($cases));
        foreach ($cases as $c) {
            $this->core->answer = fn(array $request): array => $c['problems'];
            $json = CatalogStore::encode($c['document']);
            self::assertSame($c['problems'], $validator->problems($c['kind'], $c['id'], json_decode($json, true), json_decode($json)), $c['name']);
            $sent = $this->core->requests[count($this->core->requests) - 1];
            self::assertSame('POST ' . FakeCore::URL . '/validate', $sent['url']);
            self::assertSame(FakeCore::TOKEN, $sent['headers']['X-Build-Token']);
            $expected = ['kind' => $c['kind'], 'id' => $c['id'], 'document' => $c['document'], 'instruments' => $c['instruments'], 'presets' => $c['presets']];
            self::assertEquals(self::normal($expected), self::normal($sent['request']), $c['name'] . ': the document with what it names, by value');
        }
        self::assertCount(count($cases), $this->core->requests, 'one request per check');
    }

    public function testTheCatalogsOwnRulesStayWithoutTheCore(): void
    {
        $validator = $this->get(CatalogValidator::class);
        $rig = '{"subject":"character.girl","track":[]}';
        self::assertSame(['rig character.boy: subject must be "character.boy"'], $validator->problems('rigs', 'character.boy', json_decode($rig, true), json_decode($rig), false));
        self::assertContains('id "character.Boy" is not a valid rigs id', $validator->problems('rigs', 'character.Boy', [], null, false));
        self::assertContains('id "m:01ARZ3NDEKTSV4RRFFQ69G5FAV" is not a valid presets id', $validator->problems('presets', 'm:01ARZ3NDEKTSV4RRFFQ69G5FAV', ['id' => 'm:01ARZ3NDEKTSV4RRFFQ69G5FAV', 'instrument' => 'drone'], null, false));
        self::assertSame([], $this->core->requests, 'a draft check never asks the core');
    }

    /** the acceptance of #313: a creator's publish of a broken instrument shows the core's message (a browser run is not needed) */
    public function testAPublishOfABrokenInstrumentShowsTheCoresMessage(): void
    {
        $writer = $this->creator('writer@example.com');
        $broken = ['id' => 'choir', 'kind' => 'voice', 'out' => 'sway', 'node' => [['id' => 'voice', 'kind' => 'fm'], ['id' => 'sway', 'kind' => 'lfo']]];
        $saved = $this->request('PUT', '/api/entities/instrument:choir', $writer, ['content' => $broken]);
        self::assertSame(200, $saved->getStatusCode(), (string) $saved->getBody());
        self::assertSame([], $this->core->requests, 'a draft is saved without the core');

        $message = "instrument choir: out 'sway' is not an audio output";
        $this->core->answer = fn(array $request): array => $request['kind'] === 'instruments' && ($request['document']['out'] ?? null) === 'sway' ? [$message] : [];
        $published = $this->request('POST', '/api/entities/instrument:choir/publish', $writer, (object) []);
        $body = self::json($published);
        self::assertSame([422, 'invalid_catalog', [$message], $message], [$published->getStatusCode(), $body['error'] ?? null, $body['problems'] ?? null, $body['message'] ?? null], (string) $published->getBody());
        self::assertNull($this->row('instruments', 'choir'), 'not in the catalog');
        self::assertSame('draft', $this->db->value("SELECT status FROM entity_versions WHERE entity_key = 'instrument:choir' ORDER BY id DESC LIMIT 1"), 'rolled back');

        // fixed: the next draft publishes
        $fixed = ['out' => 'voice'] + $broken;
        self::assertSame(200, $this->request('PUT', '/api/entities/instrument:choir', $writer, ['content' => $fixed])->getStatusCode());
        self::assertSame(200, $this->request('POST', '/api/entities/instrument:choir/publish', $writer, (object) [])->getStatusCode());
        self::assertNotNull($this->row('instruments', 'choir'));
    }

    public function testWithoutTheServiceNothingIsWrittenButDraftsAreSaved(): void
    {
        $doc = json_decode('{"id":"drone.dark","instrument":"drone","parent":"drone.base","values":{"drone.freq":41.0}}');
        $admin = $this->get(CatalogAdmin::class);
        $refused = function (string $status) use ($admin, $doc): void {
            try {
                $admin->put('presets', 'drone.dark', $doc, self::BY);
                self::fail("a write without the core's check ($status)");
            } catch (ApiError $e) {
                self::assertSame($status, $e->status . ' ' . $e->errorCode, $e->getMessage());
            }
            self::assertNull($this->row('presets', 'drone.dark'));
        };
        $this->core->answer = fn(array $request) => new RuntimeException('connection refused');
        $refused('503 build_unavailable');
        $this->core->answer = fn(array $request): array => ['status' => 503, 'body' => '{"error":"busy"}', 'headers' => []];
        $refused('503 build_unavailable');
        $this->core->answer = fn(array $request): array => ['status' => 401, 'body' => '{"error":"unauthorized"}', 'headers' => []];
        $refused('502 build_failed');
        $this->core->answer = fn(array $request): array => ['status' => 200, 'body' => '{"problems":"none"}', 'headers' => []];
        $refused('502 build_failed');
        // up again: its answer decides
        $this->core->answer = fn(array $request): array => [];
        $admin->put('presets', 'drone.dark', $doc, self::BY);
        self::assertNotNull($this->row('presets', 'drone.dark'));
        $sent = $this->core->requests[count($this->core->requests) - 1]['request'];
        self::assertSame([['drone'], ['drone.base']], [array_column($sent['instruments'], 'id'), array_column($sent['presets'], 'id')], 'its instrument and its parent, by value');
        // other kinds never ask
        $before = count($this->core->requests);
        $camera = json_decode((string) $this->db->value("SELECT data FROM catalog_items WHERE id = 'camera'"));
        $admin->put('items', 'camera', $camera, self::BY);
        self::assertCount($before, $this->core->requests);
    }

    public function testNotSetUpIsA503AndADraftStillSaves(): void
    {
        $this->withService = false;
        $writer = $this->creator('writer@example.com');
        $preset = ['id' => 'drone.dark', 'instrument' => 'drone', 'parent' => 'drone.base'];
        self::assertSame(200, $this->request('PUT', '/api/entities/preset:drone.dark', $writer, ['content' => $preset])->getStatusCode(), 'drafts stay editable');
        $published = $this->request('POST', '/api/entities/preset:drone.dark/publish', $writer, (object) []);
        self::assertSame([503, 'no_build_service'], [$published->getStatusCode(), self::json($published)['error'] ?? null]);
        self::assertNull($this->row('presets', 'drone.dark'));
        self::assertSame([], $this->core->requests);
    }

    // ── Helpers ───────────────────────────────────────────────────────────────────────────────────────────

    /** the table's entry now (the requests run in their own container; CatalogStore keeps what it read) */
    private function row(string $kind, string $id): ?string
    {
        $data = $this->db->value('SELECT data FROM ' . CatalogStore::table($kind) . ' WHERE id = ?', [$id]);
        return $data === null ? null : (string) $data;
    }

    /** @return array<string, mixed> */
    private function creator(string $email): array
    {
        $user = $this->user($email);
        $this->get(Roles::class)->grant((string) $user['id'], Roles::CREATOR);
        return $user;
    }

    /** @param array<string, mixed> $user */
    private function request(string $method, string $uri, array $user, mixed $body): ResponseInterface
    {
        $request = (new ServerRequestFactory())->createServerRequest($method, "http://localhost$uri", ['REMOTE_ADDR' => '127.0.0.1'])
            ->withHeader('Authorization', 'Bearer ' . $this->get(Sessions::class)->create((string) $user['id']))
            ->withHeader('Content-Type', 'application/json');
        $request->getBody()->write((string) json_encode($body, JSON_PRESERVE_ZERO_FRACTION));
        $request->getBody()->rewind();
        $container = Kernel::container(Settings::fixed(self::root(), $this->settings()));
        $container->set(Clock::class, $this->clock);
        $container->set(HttpClient::class, $this->core);
        return Kernel::app($container)->handle($request);
    }

    /** @return array<string, mixed> */
    private static function json(ResponseInterface $response): array
    {
        return (array) json_decode((string) $response->getBody(), true);
    }
}
