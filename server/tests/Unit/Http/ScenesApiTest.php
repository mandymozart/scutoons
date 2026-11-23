<?php
declare(strict_types=1);

namespace Scutoons\Tests\Http;

use Psr\Http\Message\ResponseInterface;
use Scutoons\Domain\Auth\ApiTokens;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Console\ContentFiles;
use Scutoons\Domain\Catalog\CatalogStore;
use Scutoons\Domain\Content\CollectionDocs;
use Scutoons\Domain\Content\EntityKey;
use Scutoons\Domain\Content\EntityStore;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Infrastructure\AssetStorage\HttpClient;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Kernel;
use Scutoons\Tests\FakeCore;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

/**
 * Deck scenes `scene:<id>` (#243, research/SCENES.md §3): drafts with stamped versions by the content editors, the core's
 * check before a publish, published scenes public. The fixture is the first real scene, `scene:nave` (deck-space
 * scene_document.rs writes it and checks it round-trips).
 */
final class ScenesApiTest extends TestCase
{
    private FakeCore $core;

    protected function settings(): array
    {
        return FakeCore::settings() + parent::settings();
    }

    protected function setUp(): void
    {
        parent::setUp();
        $this->core = new FakeCore();
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
        $container->set(HttpClient::class, $this->core);
        return Kernel::app($container)->handle($request);
    }

    /** @return array<string, mixed> */
    private static function json(ResponseInterface $response): array
    {
        return (array) json_decode((string) $response->getBody(), true);
    }

    /**
     * @param list<string> $roles
     * @return array<string, string>
     */
    private function session(array $roles, string $email): array
    {
        $user = $this->user($email);
        foreach ($roles as $role) $this->get(Roles::class)->grant((string) $user['id'], $role);
        return ['Authorization' => 'Bearer ' . $this->get(Sessions::class)->create((string) $user['id'])];
    }

    /** @return array<string, mixed> the first real scene as stored (no version: the server stamps it) */
    private static function nave(): array
    {
        return self::scene('nave');
    }

    /** @return array<string, mixed> a fixture of tests/fixtures/scenes (`templates/stage`, `nave.format-1`) */
    private static function scene(string $name): array
    {
        return (array) json_decode((string) file_get_contents(self::root() . "/tests/fixtures/scenes/$name.json"), true);
    }

    /**
     * #489: the four templates (research/SCENES.md §9.5) are format-2 documents the API saves – created in production once,
     * by hand, like scene:nave (docs/API.md "Scenes"; RULES E3: no file seed)
     */
    public function testTheTemplatesSaveAsScenes(): void
    {
        $creator = $this->session([Roles::CREATOR], 'templates@example.test');
        foreach (['place', 'solo', 'stage', 'avatar-home'] as $id) {
            $r = $this->request('PUT', "/api/scenes/$id", $creator, ['content' => self::scene("templates/$id")]);
            self::assertSame(200, $r->getStatusCode(), "$id: " . $r->getBody());
            $draft = self::json($this->request('GET', "/api/scenes/$id?draft=1", $creator))['content'];
            self::assertSame([$id, [], 1], [$draft['id'], $draft['toons'], $draft['version']]);
        }
    }

    public function testCreatorsSaveDraftsWithVersionsAndOnlyThePublishedOneIsPublic(): void
    {
        $creator = $this->session([Roles::CREATOR], 'creator@example.test');
        $reader = $this->session([], 'reader@example.test');

        self::assertSame(401, $this->request('PUT', '/api/scenes/nave', [], ['content' => self::nave()])->getStatusCode());
        self::assertSame(403, $this->request('PUT', '/api/scenes/nave', $reader, ['content' => self::nave()])->getStatusCode(), 'a reader saves nothing');
        $wrong = $this->request('PUT', '/api/entities/scene:nave', $creator, ['content' => self::nave()]);
        self::assertSame([400, 'wrong_route'], [$wrong->getStatusCode(), self::json($wrong)['error']], 'only /api/scenes stamps and checks a scene');

        $r = $this->request('PUT', '/api/scenes/nave', $creator, ['content' => self::nave()]);
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
        $draft = self::json($this->request('GET', '/api/scenes/nave?draft=1', $creator))['content'];
        self::assertSame(1, $draft['version']);
        self::assertSame('Nave', $draft['stage']['name']);

        // an unchanged document keeps its number, a changed one gets the next
        $this->request('PUT', '/api/scenes/nave', $creator, ['content' => self::nave()]);
        self::assertSame(1, self::json($this->request('GET', '/api/scenes/nave?draft=1', $creator))['content']['version']);
        $changed = self::nave();
        $changed['stage']['name'] = 'The nave';
        $this->request('PUT', '/api/scenes/nave', $creator, ['content' => $changed]);
        self::assertSame(2, self::json($this->request('GET', '/api/scenes/nave?draft=1', $creator))['content']['version']);
        self::assertSame('Nave', self::json($this->request('GET', '/api/scenes/nave?v=1', $creator))['scene']['stage']['name']);
        self::assertCount(2, self::json($this->request('GET', '/api/scenes/nave/versions', $creator))['versions'], 'an unchanged save stores nothing');

        // nothing published: a draft never leaves the editor
        self::assertSame(404, $this->request('GET', '/api/scenes/nave')->getStatusCode());
        self::assertSame(404, $this->request('GET', '/api/scenes/nave?v=1')->getStatusCode());
        self::assertSame(401, $this->request('GET', '/api/scenes/nave?draft=1')->getStatusCode());
        self::assertSame(403, $this->request('GET', '/api/scenes/nave/versions', $reader)->getStatusCode());
        self::assertSame([], self::json($this->request('GET', '/api/scenes'))['scenes']);
        self::assertSame('scene:nave', self::json($this->request('GET', '/api/scenes', $creator))['scenes'][0]['key']);

        $first = self::json($this->request('GET', '/api/scenes/nave/versions', $creator))['versions'][1]['id'];
        self::assertSame(200, $this->request('POST', '/api/scenes/nave/publish', $creator, ['version' => $first])->getStatusCode());
        $public = self::json($this->request('GET', '/api/scenes/nave'));
        self::assertSame(['nave', 1, 'Nave'], [$public['scene']['id'], $public['scene']['version'], $public['scene']['stage']['name']]);
        self::assertSame(1, self::json($this->request('GET', '/api/scenes/nave?v=1'))['scene']['version']);
        self::assertSame(404, $this->request('GET', '/api/scenes/nave?v=2')->getStatusCode(), 'v2 is a draft');
        $listed = self::json($this->request('GET', '/api/scenes', $reader))['scenes'];
        self::assertSame([['key' => 'scene:nave', 'content' => $public['scene']]], $listed, 'readers see the published document, not who saved it');

        // restore: the old content as the newest draft, stamped anew (it differs from v2: v3)
        $restored = $this->request('POST', '/api/scenes/nave/restore', $creator, ['id' => $first]);
        self::assertSame(200, $restored->getStatusCode(), (string) $restored->getBody());
        $draft = self::json($this->request('GET', '/api/scenes/nave?draft=1', $creator))['content'];
        self::assertSame([3, 'Nave'], [$draft['version'], $draft['stage']['name']]);
    }

    /** #687: the History's reverts – a new draft, stamped anew and checked like a save; a stored version read by its id */
    public function testARevertIsANewStampedDraftCheckedLikeASave(): void
    {
        $creator = $this->session([Roles::CREATOR], 'revert@example.test');
        $ids = [];
        $base = null;
        foreach (['Nave', 'The nave'] as $name) {
            $doc = self::nave();
            $doc['stage']['name'] = $name;
            $base = self::json($this->request('PUT', '/api/scenes/nave', $creator, ['content' => $doc, 'base' => $base]))['version']['id'];
            $ids[] = $base;
        }
        self::assertSame('Nave', self::json($this->request('GET', "/api/scenes/nave?draft=1&at=$ids[0]", $creator))['content']['stage']['name']);

        $done = self::json($this->request('POST', '/api/scenes/nave/revert', $creator, ['version' => $ids[1], 'content' => self::nave(), 'base' => $ids[1]]));
        self::assertSame("revert #$ids[1]", $done['version']['message']);
        $now = self::json($this->request('GET', '/api/scenes/nave?draft=1', $creator))['content'];
        self::assertSame(['Nave', 3], [$now['stage']['name'], $now['version']]);
        self::assertSame(422, $this->request('POST', '/api/scenes/nave/revert', $creator, ['version' => $ids[1], 'content' => ['id' => 'nave'], 'base' => $done['version']['id']])->getStatusCode());

        $back = self::json($this->request('POST', '/api/scenes/nave/revert-to', $creator, ['version' => $ids[1], 'base' => $done['version']['id']]));
        self::assertSame("back to #$ids[1]", $back['version']['message']);
        $now = self::json($this->request('GET', '/api/scenes/nave?draft=1', $creator))['content'];
        self::assertSame(['The nave', 4], [$now['stage']['name'], $now['version']]);
        self::assertSame(409, $this->request('POST', '/api/scenes/nave/revert-to', $creator, ['version' => $ids[0], 'base' => $ids[1]])->getStatusCode());
    }

    public function testTheShapeIsCheckedOnSave(): void
    {
        $creator = $this->session([Roles::CREATOR], 'shape@example.test');
        $other = self::nave();
        $other['id'] = 'other';
        self::assertSame(422, $this->request('PUT', '/api/scenes/nave', $creator, ['content' => $other])->getStatusCode(), 'the document names itself');
        foreach ([
            // #489: format 1 was moved by the migration 2026-11-12-scene-format-2 and is refused from now on
            'format 1' => fn(array $d) => self::scene('nave.format-1'),
            'an unknown field' => fn(array $d) => ['nodes' => []] + $d,
            'the being (it left the document: a character holder brings its look)' => fn(array $d) => $d + ['being' => ['size' => 0.5]],
            'a holder that takes some' => function (array $d): array {
                $d['holder'][0]['take'] = 'some';
                return $d;
            },
            'a filter of a kind no holder takes' => function (array $d): array {
                $d['holder'][] = ['id' => 'who', 'fill' => ['kind' => ['chapter']]];
                return $d;
            },
        ] as $what => $change) {
            $r = $this->request('PUT', '/api/scenes/nave', $creator, ['content' => $change(self::nave())]);
            self::assertSame(422, $r->getStatusCode(), "$what: " . $r->getBody());
            self::assertSame('invalid_content', self::json($r)['error'], $what);
        }
        self::assertSame([], $this->core->requests, 'a save never asks the core');
    }

    /** #803: "publish everything" (the partial publish) takes deck scenes too – last, through the core's scene check */
    public function testThePartialPublishTakesAScene(): void
    {
        $creator = $this->session([Roles::CREATOR], 'creator@example.test');
        self::assertSame(200, $this->request('PUT', '/api/scenes/nave', $creator, ['content' => self::nave()])->getStatusCode());
        $publish = fn() => self::json($this->request('POST', '/api/entities/publish', $creator, ['versions' => [['key' => 'scene:nave']], 'partial' => true]));
        $this->core->answer = fn(array $request): array => ["node 'a': unknown kind 'blender'"];
        $r = $publish();
        self::assertSame([], $r['published']);
        self::assertSame([['key' => 'scene:nave', 'code' => 'invalid_scene', 'reason' => "node 'a': unknown kind 'blender'"]], $r['refused'], 'the core words');
        self::assertSame('scenes', $this->core->requests[count($this->core->requests) - 1]['request']['kind']);
        $this->core->answer = fn(array $request): array => [];
        $r = $publish();
        self::assertSame(['scene:nave'], array_column($r['published'], 'key'));
        self::assertSame([], $r['refused']);
        self::assertSame('Nave', self::json($this->request('GET', '/api/scenes/nave'))['scene']['stage']['name']);
    }

    /** #803: a scene's Publish takes the draft rig it plays along, first; a refused rig skips the scene with the reason */
    public function testAScenesPublishTakesTheDraftsItPlays(): void
    {
        $creator = $this->session([Roles::CREATOR], 'creator@example.test');
        self::assertSame(200, $this->request('PUT', '/api/scenes/nave', $creator, ['content' => self::nave()])->getStatusCode());
        self::assertSame(200, $this->request('PUT', '/api/entities/rig:character.catpope', $creator, ['content' => ['subject' => 'character.catpope', 'track' => []]])->getStatusCode());
        $plan = self::json($this->request('POST', '/api/entities/publish/plan', $creator, ['versions' => [['key' => 'scene:nave']]]));
        self::assertSame(['rig:character.catpope' => 'scene:nave'], array_column($plan['entities'], 'needed_by', 'key'));
        $publish = fn() => self::json($this->request('POST', '/api/entities/publish', $creator, ['versions' => [['key' => 'scene:nave']], 'partial' => true, 'with_dependencies' => true]));

        $this->core->answer = fn(array $request): array => $request['kind'] === 'rigs' ? ['rig character.catpope: no track'] : [];
        $r = $publish();
        self::assertSame([], $r['published']);
        self::assertSame(['rig:character.catpope' => 'rig character.catpope: no track', 'scene:nave' => 'needs rig character.catpope, which was refused'], array_column($r['refused'], 'reason', 'key'));

        $this->core->answer = fn(array $request): array => [];
        $r = $publish();
        self::assertSame(['rig:character.catpope', 'scene:nave'], array_column($r['published'], 'key'), 'the rig first, the scene last');
        self::assertSame([['key' => 'rig:character.catpope', 'needed_by' => 'scene:nave']], $r['added']);
    }

    public function testPublishingAsksTheCoreAndRefusesWhatADeckCouldNotPlay(): void
    {
        $creator = $this->session([Roles::CREATOR], 'publish@example.test');
        $this->request('PUT', '/api/scenes/nave', $creator, ['content' => self::nave()]);

        $this->core->answer = fn(array $request): array => ["node 'a': unknown kind 'blender'"];
        $r = $this->request('POST', '/api/scenes/nave/publish', $creator, []);
        self::assertSame(422, $r->getStatusCode());
        self::assertSame('invalid_scene', self::json($r)['error']);
        self::assertSame(["node 'a': unknown kind 'blender'"], self::json($r)['problems']);
        $asked = $this->core->requests[0]['request'];
        self::assertSame(['scenes', 'nave', 'nave'], [$asked['kind'], $asked['id'], $asked['document']['id']]);
        self::assertSame(404, $this->request('GET', '/api/scenes/nave')->getStatusCode(), 'nothing published');

        $this->core->answer = fn(array $request): array => [];
        self::assertSame(200, $this->request('POST', '/api/scenes/nave/publish', $creator, [])->getStatusCode());
        self::assertSame(200, $this->request('GET', '/api/scenes/nave')->getStatusCode());
    }

    /**
     * A scene's module nodes – the publish sends the catalog's descriptors of the modules they name; one the catalog
     * lacks is not sent and the core's sentence naming it refuses the scene (fixtures/scenes/modules.json, the core's vectors)
     */
    public function testPublishingSendsTheDescriptorsOfTheScenesModules(): void
    {
        $creator = $this->session([Roles::CREATOR], 'modules@example.test');
        $set = (array) json_decode((string) file_get_contents(self::root() . '/tests/fixtures/scenes/modules.json'), true);
        $analog = json_decode((string) file_get_contents(self::root() . '/tests/fixtures/modules/service-answer-analog.json'))->descriptor;
        foreach ($set['cases'] as $case) {
            $this->db->exec('DELETE FROM catalog_modules');
            foreach ($case['catalog'] as $id) $this->get(CatalogStore::class)->save('modules', $id, (object) ['id' => $id, 'name' => $id, 'descriptor' => $analog], 'test');
            $saved = $this->request('PUT', '/api/scenes/rave', $creator, ['content' => $case['request']['document']]);
            self::assertSame(200, $saved->getStatusCode(), "{$case['name']}: " . $saved->getBody());
            $this->core->answer = fn(array $request): array => $case['problems'];
            $r = $this->request('POST', '/api/scenes/rave/publish', $creator, []);
            $asked = $this->core->requests[count($this->core->requests) - 1]['request'];
            unset($asked['document']['version']);
            $expected = $case['request'];
            if ($expected['modules'] === []) unset($expected['modules']);
            self::assertEquals($expected, $asked, "{$case['name']}: the request the core's vectors answer");
            if ($case['problems'] === []) {
                self::assertSame(200, $r->getStatusCode(), "{$case['name']}: " . $r->getBody());
            } else {
                self::assertSame([422, 'invalid_scene', $case['problems']], [$r->getStatusCode(), self::json($r)['error'] ?? null, self::json($r)['problems'] ?? null], $case['name']);
            }
        }
    }

    /**
     * #815 (crows-rave v981): the check gets the rigs a deck plays – scene-content's answer for what a deck asks (each
     * filler's own rig, each pinned one); a crow without a rig of its own has none in it (the core plays the default
     * character rig for it), Cat Pope and the incense burner have theirs (the migrations' rows)
     */
    public function testPublishingSendsTheRigsTheHoldersPlay(): void
    {
        $store = $this->get(EntityStore::class);
        $registry = ['characters' => [
            ['id' => 'crow-wolf', 'personas' => [['id' => 'regular', 'looks' => [['id' => 'default', 'default' => true, 'images' => []]]]]],
            ['id' => 'catpope', 'personas' => [['id' => 'regular', 'looks' => [['id' => 'default', 'default' => true, 'images' => []]]]]],
        ]];
        $collection = CollectionDocs::ofAnchor((string) ContentFiles::keyOf('scutoons', '_', 'characters.json'));
        self::assertNotNull($collection);
        foreach (CollectionDocs::split($collection, json_decode((string) json_encode($registry))) as $part => $doc) {
            $id = $store->insert(EntityKey::parse((string) $part), (string) json_encode($doc), null, 'draft', null);
            $store->setPublished(['id' => $id, 'entity_key' => (string) $part]);
        }
        $creator = $this->session([Roles::CREATOR], 'rigs@example.test');
        $rave = ['format' => 2, 'id' => 'rave', 'toons' => [], 'holder' => [
            ['id' => 'wolf', 'ref' => 'character:crow-wolf', 'values' => ['voice:voice.gain' => 0.0]],
            ['id' => 'pope', 'ref' => 'character:boy', 'rig' => ['ref' => 'rig:character.catpope']],
            ['id' => 'air', 'ref' => 'item:incense_burner'],
        ], 'stage' => ['name' => 'Rave'], 'node' => []];
        self::assertSame(200, $this->request('PUT', '/api/scenes/rave', $creator, ['content' => $rave])->getStatusCode());
        $this->core->answer = fn(array $request): array => [];
        self::assertSame(200, $this->request('POST', '/api/scenes/rave/publish', $creator, [])->getStatusCode());
        $content = $this->core->requests[count($this->core->requests) - 1]['request']['content'];
        self::assertEqualsCanonicalizing(['catpope', 'crow-wolf'], array_keys($content['characters']), 'a filler\'s own rig and a pinned one (not boy\'s: its holder pins catpope\'s), as a deck asks');
        self::assertNull($content['characters']['crow-wolf']['rig'], 'no rig of its own: the core plays the default character rig');
        self::assertSame('character.catpope', $content['characters']['catpope']['rig']['id']);
        self::assertSame('item.incense_burner', $content['items']['incense_burner']['rig']['id']);
        self::assertSame(['cathedral_voice', 'drone', 'incense'], array_keys($content['instruments']), 'what the rigs name, once each');

        // a scene without holders asks for no rigs: none sent
        $empty = ['format' => 2, 'id' => 'quiet', 'toons' => [], 'stage' => ['name' => 'Quiet'], 'node' => []];
        $this->request('PUT', '/api/scenes/quiet', $creator, ['content' => $empty]);
        self::assertSame(200, $this->request('POST', '/api/scenes/quiet/publish', $creator, [])->getStatusCode());
        self::assertArrayNotHasKey('content', $this->core->requests[count($this->core->requests) - 1]['request']);
    }

    /**
     * a placed rig (a holder with `rig` alone, deck-space scene_document.rs) – the drum machine of
     * tests/fixtures/rigs/drum-machine.json: the shape takes it, publishing sends its rig and the descriptors of the
     * modules its instruments play (the core builds the graph with them)
     */
    public function testAPlacedRigSavesAndItsRigGoesToTheCore(): void
    {
        $store = $this->get(CatalogStore::class);
        $bundle = json_decode((string) file_get_contents(self::root() . '/tests/fixtures/rigs/drum-machine.json'));
        foreach ($bundle->instrument as $i) $store->save('instruments', $i->id, $i, 'test');
        $store->save('rigs', 'item.drum-machine', $bundle->rig, 'test');
        $analog = json_decode((string) file_get_contents(self::root() . '/tests/fixtures/modules/service-answer-analog.json'))->descriptor;
        $store->save('modules', 'analog', (object) ['id' => 'analog', 'name' => 'analog', 'descriptor' => $analog], 'test');
        $creator = $this->session([Roles::CREATOR], 'placed@example.test');
        $scene = ['format' => 2, 'id' => 'placed', 'toons' => [], 'holder' => [
            ['id' => 'drum-machine', 'at' => [2.25, 0.0, 0.0], 'rig' => ['ref' => 'rig:item.drum-machine']],
        ], 'stage' => ['name' => 'Placed'], 'node' => []];
        $saved = $this->request('PUT', '/api/scenes/placed', $creator, ['content' => $scene]);
        self::assertSame(200, $saved->getStatusCode(), (string) $saved->getBody());
        $this->core->answer = fn(array $request): array => [];
        self::assertSame(200, $this->request('POST', '/api/scenes/placed/publish', $creator, [])->getStatusCode());
        $request = $this->core->requests[count($this->core->requests) - 1]['request'];
        self::assertSame('item.drum-machine', $request['content']['items']['drum-machine']['rig']['id']);
        self::assertSame('characters/takeyari-drum/takeyari-drum-regular.glb', $request['content']['items']['drum-machine']['body']['path']);
        self::assertSame(['analog'], array_column($request['modules'] ?? [], 'name'), 'its voices\' module (drumseq is not in this catalog: left out, the core names it)');
    }

    public function testWithoutTheBuildServiceNothingIsPublished(): void
    {
        $creator = $this->session([Roles::CREATOR], 'nobuild@example.test');
        $this->request('PUT', '/api/scenes/nave', $creator, ['content' => self::nave()]);
        $this->core->answer = fn(array $request): \RuntimeException => new \RuntimeException('down');
        self::assertSame(503, $this->request('POST', '/api/scenes/nave/publish', $creator, [])->getStatusCode());
    }

    public function testAnAiConnectionSavesDraftsButNeverPublishes(): void
    {
        $user = $this->user('ai@example.test');
        $this->get(Roles::class)->grant((string) $user['id'], Roles::CREATOR);
        $token = $this->get(ApiTokens::class)->create($user, 'Claude')['token'];
        $ai = ['Authorization' => "Bearer $token"];
        $r = $this->request('PUT', '/api/scenes/nave', $ai, ['content' => self::nave(), 'message' => 'first']);
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
        self::assertSame('first (AI: Claude)', self::json($r)['version']['message']);
        self::assertSame(403, $this->request('POST', '/api/scenes/nave/publish', $ai, [])->getStatusCode());
    }
}
