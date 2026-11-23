<?php
declare(strict_types=1);

namespace Scutoons\Tests\Http;

use Psr\Http\Message\ResponseInterface;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Domain\Crafts\Crafts;
use Scutoons\Domain\Layouts\WorkspaceLayouts;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Kernel;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

/**
 * Saved workspace layouts (Domain\Layouts, #827): a member saves a workspace's layout and gets it back, own workspaces
 * with a name, delete = back to the default; only the outer shape and the size are checked; own rows only; an AI
 * connection only reads; a craft's default comes with the member's crafts; before the migration nothing throws.
 */
final class LayoutsApiTest extends TestCase
{
    private const LAYOUT = [
        'places' => ['left' => ['corner' => 'top-left', 'dx' => 14, 'dy' => 0, 'size' => ['w' => 300, 'h' => 500]]],
        'tabs' => ['panels' => ['left' => ['toon', 'tree']], 'active' => ['left' => 'tree'], 'free' => []],
        'stage' => ['layout' => 'one', 'slots' => [null]],
    ];

    /** @param array<string, string> $headers */
    private function request(string $method, string $uri, array $headers = [], mixed $body = null): ResponseInterface
    {
        $request = (new ServerRequestFactory())->createServerRequest($method, "http://localhost$uri", ['REMOTE_ADDR' => '127.0.0.1']);
        foreach ($headers as $name => $value) $request = $request->withHeader($name, $value);
        if ($body !== null) {
            $request->getBody()->write((string) json_encode($body));
            $request->getBody()->rewind();
            $request = $request->withHeader('Content-Type', 'application/json');
        }
        $container = Kernel::container(Settings::fixed(self::root(), $this->settings()));
        $container->set(Clock::class, $this->clock);
        return Kernel::app($container)->handle($request);
    }

    /** @return array<string, mixed> */
    private static function json(ResponseInterface $response): array
    {
        return (array) json_decode((string) $response->getBody(), true);
    }

    /** @return array{0: array<string, string>, 1: string} */
    private function session(string $email): array
    {
        $user = $this->user($email);
        return [['Authorization' => 'Bearer ' . $this->get(Sessions::class)->create((string) $user['id'])], (string) $user['id']];
    }

    private static function error(ResponseInterface $r): string
    {
        return $r->getStatusCode() . ' ' . (self::json($r)['error'] ?? '');
    }

    public function testAWorkspaceIsSavedAndResetToItsDefault(): void
    {
        [$member] = $this->session('mira@scutoons.test');
        self::assertSame(401, $this->request('GET', '/api/me/layouts')->getStatusCode());
        $state = self::json($this->request('GET', '/api/me/layouts', $member));
        self::assertSame([true, [], []], [$state['available'], $state['layouts'], $state['defaults']]);

        $r = $this->request('PUT', '/api/me/layouts/character', $member, ['layout' => self::LAYOUT]);
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
        $saved = self::json($r)['layout'];
        self::assertSame(['character', null, null, self::LAYOUT], [$saved['workspace'], $saved['kind'], $saved['name'], $saved['layout']]);

        // saving again replaces it: one row per workspace
        $moved = ['stage' => ['layout' => 'half-left', 'slots' => ['kanon', null]]];
        self::assertSame(200, $this->request('PUT', '/api/me/layouts/character', $member, ['layout' => $moved])->getStatusCode());
        $layouts = self::json($this->request('GET', '/api/me/layouts', $member))['layouts'];
        self::assertSame([['character', $moved]], array_map(static fn(array $l) => [$l['workspace'], $l['layout']], $layouts));

        $r = $this->request('DELETE', '/api/me/layouts/character', $member);
        self::assertSame([200, true], [$r->getStatusCode(), self::json($r)['deleted']]);
        self::assertSame(false, self::json($this->request('DELETE', '/api/me/layouts/character', $member))['deleted']);
        self::assertSame([], self::json($this->request('GET', '/api/me/layouts', $member))['layouts']);
    }

    public function testAnOwnWorkspaceHasANameAndTheWorkspaceItIsMadeFrom(): void
    {
        [$member] = $this->session('own@scutoons.test');
        $r = $this->request('PUT', '/api/me/layouts/own-a1b2c3', $member, ['layout' => self::LAYOUT, 'name' => '  Looks   for 3D ', 'kind' => 'character']);
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
        self::assertSame(['character', 'Looks for 3D'], [self::json($r)['layout']['kind'], self::json($r)['layout']['name']]);
        // saved again without a name it keeps its name; renamed with one
        $this->request('PUT', '/api/me/layouts/own-a1b2c3', $member, ['layout' => []]);
        $this->request('PUT', '/api/me/layouts/own-a1b2c3', $member, ['layout' => [], 'name' => 'Expressions']);
        $row = self::json($this->request('GET', '/api/me/layouts', $member))['layouts'][0];
        self::assertSame(['own-a1b2c3', 'character', 'Expressions'], [$row['workspace'], $row['kind'], $row['name']]);

        self::assertSame('400 bad_kind', self::error($this->request('PUT', '/api/me/layouts/own-new', $member, ['layout' => [], 'name' => 'No kind'])));
        self::assertSame('400 bad_name', self::error($this->request('PUT', '/api/me/layouts/own-new', $member, ['layout' => [], 'kind' => 'chapter'])));
        self::assertSame('400 bad_kind', self::error($this->request('PUT', '/api/me/layouts/own-new', $member, ['layout' => [], 'name' => 'Me', 'kind' => 'own-new'])));
        // a creator workspace's saved state keeps no name
        $this->request('PUT', '/api/me/layouts/chapter', $member, ['layout' => []]);
        self::assertSame('400 bad_name', self::error($this->request('PUT', '/api/me/layouts/chapter', $member, ['layout' => [], 'name' => 'Mine'])));
    }

    public function testOnlyTheShapeAndTheSizeAreChecked(): void
    {
        [$member] = $this->session('shape@scutoons.test');
        foreach ([
            '400 bad_layout' => ['layout' => ['places' => [], 'colour' => []]],
            '400 bad_layout ' => ['layout' => ['places' => 'left']],
            '400 bad_layout  ' => ['layout' => [1, 2]],
            '400 bad_layout   ' => [],
            '413 layout_too_large' => ['layout' => ['places' => ['x' => str_repeat('a', WorkspaceLayouts::MAX_BYTES)]]],
            '400 bad_name' => ['layout' => [], 'name' => str_repeat('n', WorkspaceLayouts::MAX_NAME + 1), 'kind' => 'chapter'],
        ] as $expected => $body) {
            self::assertSame(trim($expected), self::error($this->request('PUT', '/api/me/layouts/toon', $member, $body)), json_encode($body) ?: '');
        }
        self::assertSame('400 bad_workspace', self::error($this->request('PUT', '/api/me/layouts/Not%20an%20id', $member, ['layout' => []])));
        // the inside is the creator's: unknown panels and places are kept as sent
        $odd = ['places' => ['nowhere' => ['corner' => 'middle']], 'tabs' => ['panels' => ['x' => ['y']]]];
        self::assertSame($odd, self::json($this->request('PUT', '/api/me/layouts/toon', $member, ['layout' => $odd]))['layout']['layout']);
    }

    public function testAMemberSeesAndChangesOnlyTheirOwn(): void
    {
        [$mira] = $this->session('mira@scutoons.test');
        [$ben] = $this->session('ben@scutoons.test');
        $this->request('PUT', '/api/me/layouts/character', $mira, ['layout' => self::LAYOUT]);
        self::assertSame([], self::json($this->request('GET', '/api/me/layouts', $ben))['layouts']);
        self::assertSame(false, self::json($this->request('DELETE', '/api/me/layouts/character', $ben))['deleted']);
        self::assertCount(1, self::json($this->request('GET', '/api/me/layouts', $mira))['layouts']);
    }

    public function testTooManySavedWorkspacesAreRefused(): void
    {
        [$member, $id] = $this->session('many@scutoons.test');
        for ($i = 0; $i < WorkspaceLayouts::MAX_ROWS; $i++) {
            $this->db->run('INSERT INTO member_layouts (user_id, workspace, kind, name, layout, saved_at) VALUES (?, ?, ?, ?, ?, ?)', [$id, "own-$i", 'chapter', "W $i", '{}', $this->clock->utc()]);
        }
        self::assertSame('409 too_many_layouts', self::error($this->request('PUT', '/api/me/layouts/toon', $member, ['layout' => []])));
        // an existing one still saves
        self::assertSame(200, $this->request('PUT', '/api/me/layouts/own-3', $member, ['layout' => []])->getStatusCode());
    }

    public function testACraftsDefaultComesWithTheMembersCrafts(): void
    {
        [$member, $id] = $this->session('craft@scutoons.test');
        $this->db->run('INSERT INTO craft_layouts (craft_id, workspace, layout, updated_at) VALUES (?, ?, ?, ?)', ['illustrator', 'character', '{"stage":{"layout":"one","slots":["kanon"]}}', $this->clock->utc()]);
        $this->db->run('INSERT INTO craft_layouts (craft_id, workspace, layout, updated_at) VALUES (?, ?, ?, ?)', ['writer', 'character', '{"places":{}}', $this->clock->utc()]);
        $this->db->run('INSERT INTO craft_layouts (craft_id, workspace, layout, updated_at) VALUES (?, ?, ?, ?)', ['writer', 'text', '{"places":{}}', $this->clock->utc()]);
        self::assertSame([], self::json($this->request('GET', '/api/me/layouts', $member))['defaults']);

        $this->get(Crafts::class)->setFor($id, ['writer', 'illustrator'], 'admin@scutoons.test');
        $defaults = self::json($this->request('GET', '/api/me/layouts', $member))['defaults'];
        // per workspace the first craft in the crafts' order (illustrator before writer)
        self::assertSame([['character', 'illustrator'], ['text', 'writer']], array_map(static fn(array $d) => [$d['workspace'], $d['craft']], $defaults));
        self::assertSame(['layout' => 'one', 'slots' => ['kanon']], $defaults[0]['layout']['stage']);
    }

    public function testAnAiConnectionOnlyReads(): void
    {
        [$member, $id] = $this->session('ai@scutoons.test');
        $this->get(Roles::class)->grant($id, 'creator');
        $token = (string) self::json($this->request('POST', '/api/me/tokens', $member, ['name' => 'Claude']))['token'];
        $ai = ['Authorization' => 'Bearer ' . $token];
        $this->request('PUT', '/api/me/layouts/character', $member, ['layout' => self::LAYOUT]);
        self::assertCount(1, self::json($this->request('GET', '/api/me/layouts', $ai))['layouts']);
        self::assertSame('403 not_for_ai', self::error($this->request('PUT', '/api/me/layouts/character', $ai, ['layout' => []])));
        self::assertSame('403 not_for_ai', self::error($this->request('DELETE', '/api/me/layouts/character', $ai)));
    }

    public function testWithoutTheMigrationNothingThrows(): void
    {
        [$member] = $this->session('early@scutoons.test');
        $this->db->exec('DROP TABLE member_layouts');
        $this->db->exec('DROP TABLE craft_layouts');
        $state = self::json($this->request('GET', '/api/me/layouts', $member));
        self::assertSame([false, [], []], [$state['available'], $state['layouts'], $state['defaults']]);
        self::assertSame('503 not_available', self::error($this->request('PUT', '/api/me/layouts/character', $member, ['layout' => []])));
        self::assertSame('503 not_available', self::error($this->request('DELETE', '/api/me/layouts/character', $member)));
    }
}
