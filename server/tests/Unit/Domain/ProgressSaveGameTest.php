<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Psr\Http\Message\ResponseInterface;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Domain\Progress\ProgressService;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Database\MigrationRunner;
use Scutoons\Kernel;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

/** The ONE save game (DATA.md "The save game"): the merge on PUT /api/progress/scutoons/ and the migration of the per-toon states */
final class ProgressSaveGameTest extends TestCase
{
    /** @param array<mixed> $state */
    private static function obj(array $state): mixed
    {
        return json_decode((string) json_encode($state));
    }

    /** @param array<string, mixed> $state */
    private function row(string $userId, string $toon, array $state, int $updatedAt): void
    {
        $this->db->run('INSERT INTO progress (user_id, toon_id, state, updated_at) VALUES (?, ?, ?, ?)', [$userId, $toon, (string) json_encode($state, JSON_UNESCAPED_SLASHES), $updatedAt]);
    }

    /** @return array<string, mixed> */
    private function stored(string $userId, string $toon = 'scutoons'): array
    {
        return (array) json_decode((string) $this->db->value('SELECT state FROM progress WHERE user_id = ? AND toon_id = ?', [$userId, $toon]), true);
    }

    /** @return array<string, mixed> */
    private static function cp(string $uri, string $updated, bool $done = false, float $position = 0.5): array
    {
        return ['uri' => $uri, 'isComplete' => $done, 'position' => $position, 'created_at' => $updated, 'updated_at' => $updated];
    }

    public function testThePutMergesCheckpointsAndHistoryAndTakesTheNewerVariables(): void
    {
        $id = (string) $this->user()['id'];
        $progress = $this->get(ProgressService::class);
        $progress->put($id, 'scutoons', self::obj([
            'variables' => ['tips' => 1, 'old' => 'x'],
            'progress' => [self::cp('catpope/chapter/1', '2026-01-02T00:00:00.000Z', true, 1)],
            'history' => ['event:a' => ['n' => 3, 'first' => 10, 'last' => 30]],
            'map' => ['places' => ['harbour' => ['unlocked' => 5]]],
            'created_at' => '2026-01-01T00:00:00.000Z',
        ]), 2000);
        $r = $progress->put($id, 'scutoons', self::obj([
            'variables' => ['tips' => 2],
            'progress' => [self::cp('toptoy/chapter/1', '2026-01-03T00:00:00.000Z'), self::cp('catpope/chapter/1', '2026-01-04T00:00:00.000Z', false, 0.1)],
            'history' => ['event:a' => ['n' => 1, 'first' => 20, 'last' => 40], 'event:b' => ['n' => 1, 'first' => 1, 'last' => 1]],
            'map' => ['places' => ['market' => ['unlocked' => 7]]],
            'created_at' => '2026-01-05T00:00:00.000Z',
        ]), 3000);
        self::assertSame('success', $r['status']);
        $s = $this->stored($id);
        self::assertSame(['tips' => 2], $s['variables'], 'variables: the newer write');
        $byUri = array_column($s['progress'], null, 'uri');
        self::assertEqualsCanonicalizing(['catpope/chapter/1', 'toptoy/chapter/1'], array_keys($byUri));
        self::assertSame([true, 0.1], [$byUri['catpope/chapter/1']['isComplete'], $byUri['catpope/chapter/1']['position']], 'the later checkpoint, finished stays');
        self::assertSame(['n' => 3, 'first' => 10, 'last' => 40], $s['history']['event:a']);
        self::assertArrayHasKey('event:b', $s['history']);
        self::assertEqualsCanonicalizing(['harbour', 'market'], array_keys($s['map']['places']));
        self::assertSame('2026-01-01T00:00:00.000Z', $s['created_at'], 'the earlier start');
        // a stale write: its checkpoints / history still count, its variables not
        $r = $progress->put($id, 'scutoons', self::obj(['variables' => ['tips' => 0], 'progress' => [self::cp('yellowmouse/chapter/1', '2026-01-01T00:00:00.000Z')]]), 1000);
        self::assertSame('stale', $r['status']);
        $s = $this->stored($id);
        self::assertSame(['tips' => 2], $s['variables']);
        self::assertCount(3, $s['progress']);
        // an old world state without variables (map only, an older viewer build) keeps the stored variables
        $progress->put($id, 'scutoons', self::obj(['map' => ['places' => ['pond' => ['unlocked' => 9]]]]), 4000);
        self::assertSame(['tips' => 2], $this->stored($id)['variables']);
    }

    /** "Reset the map" (owner 2026-10-11): what came before the reset drops out – also from a device that missed it */
    public function testAMapResetDropsWhatCameBeforeItAlsoFromAStaleDevice(): void
    {
        $id = (string) $this->user()['id'];
        $progress = $this->get(ProgressService::class);
        $old = ['places' => ['harbour' => ['unlocked' => 100, 'seen' => 110]], 'routes' => ['season1' => ['revealed' => 2]], 'positions' => ['catpope' => ['location' => 'harbour', 'at' => 120]]];
        $progress->put($id, 'scutoons', self::obj(['variables' => ['tips' => 1], 'map' => $old]), 2000);
        // the reset (this device), then a stale phone that still has the old map and found the well after the reset
        $progress->put($id, 'scutoons', self::obj(['variables' => ['tips' => 1], 'map' => ['places' => (object) [], 'routes' => (object) [], 'positions' => (object) [], 'resetAt' => 1000]]), 3000);
        $s = $this->stored($id);
        self::assertSame([], $s['map']['places']);
        self::assertSame([], $s['map']['routes']);
        self::assertSame(1000, $s['map']['resetAt']);
        $phone = $old;
        $phone['places']['well'] = ['unlocked' => 1500];
        $progress->put($id, 'scutoons', self::obj(['variables' => ['tips' => 1], 'map' => $phone]), 2500);
        $s = $this->stored($id);
        self::assertSame(['well'], array_keys($s['map']['places']), 'the old place stays gone, the new one counts');
        self::assertSame([], $s['map']['routes'], 'the phone\'s routes are from before the reset');
        self::assertSame([], $s['map']['positions']);
        self::assertSame(['tips' => 1], $s['variables'], 'only the map');
        // without a reset nothing changes
        $plain = ProgressService::mergeMapState(self::obj($old), self::obj($old));
        self::assertObjectNotHasProperty('resetAt', $plain);
        self::assertSame(['catpope'], array_keys(get_object_vars($plain->positions)));
    }

    /** the API: the save game under /api/progress/scutoons/, the per-toon calls of older viewer builds still answer */
    public function testTheApi(): void
    {
        $user = $this->user();
        $token = $this->get(Sessions::class)->create((string) $user['id']);
        $call = function (string $method, string $uri, ?array $body = null) use ($token): ResponseInterface {
            $request = (new ServerRequestFactory())->createServerRequest($method, "http://localhost$uri", ['REMOTE_ADDR' => '127.0.0.1'])
                ->withHeader('Authorization', "Bearer $token")->withHeader('Content-Type', 'application/json');
            if ($body !== null) $request->getBody()->write((string) json_encode($body));
            $container = Kernel::container(Settings::fixed(self::root(), $this->settings()));
            $container->set(Clock::class, $this->clock);
            return Kernel::app($container)->handle($request);
        };
        $put = $call('PUT', '/api/progress/scutoons/', ['state' => ['variables' => ['mama_tips' => 2], 'progress' => [], 'history' => (object) [], 'map' => (object) []], 'updated_at' => 5000]);
        self::assertSame(200, $put->getStatusCode(), (string) $put->getBody());
        $get = json_decode((string) $call('GET', '/api/progress/scutoons/')->getBody(), true);
        self::assertSame(['mama_tips' => 2], $get['state']['variables']);
        self::assertSame(5000, $get['updated_at']);
        // an older build's per-toon state: still stored and answered (last write wins), not mixed into the save game
        self::assertSame(200, $call('PUT', '/api/progress/catpope/', ['state' => ['toonId' => 'catpope', 'variables' => ['x' => 1]], 'updated_at' => 6000])->getStatusCode());
        self::assertSame(['x' => 1], json_decode((string) $call('GET', '/api/progress/catpope/')->getBody(), true)['state']['variables']);
        self::assertSame(['mama_tips' => 2], $this->stored((string) $user['id'])['variables']);
    }

    /**
     * 2026-10-06-global-save-game: each member's per-toon states merged into the save game – a toon's variables keep
     * their domain (`tips` of catpope → `catpope/tips`, @scutoons/content variables.ts: no clashes between toons);
     * checkpoints of every toon; the map stays; a second run changes nothing;
     * the per-toon rows stay; members without per-toon states are not touched
     */
    public function testTheGlobalSaveGameMigration(): void
    {
        $a = (string) $this->user()['id'];
        $b = (string) $this->user()['id'];
        $c = (string) $this->user()['id'];
        $this->row($a, 'catpope', ['toonId' => 'catpope', 'variables' => ['tips' => 2, 'visits_harbour' => 3], 'progress' => [self::cp('catpope/chapter/2', '2026-02-01T00:00:00.000Z', true, 1)],
            'history' => ['event:c' => ['n' => 1, 'first' => 1, 'last' => 1]], 'created_at' => '2026-01-01T00:00:00.000Z', 'updated_at' => '2026-02-01T00:00:00.000Z'], 2000);
        $this->row($a, 'toptoy', ['toonId' => 'toptoy', 'variables' => ['tips' => 5, 'visits_hypercity' => 1], 'progress' => [self::cp('toptoy/chapter/1', '2026-01-15T00:00:00.000Z')],
            'created_at' => '2026-01-10T00:00:00.000Z', 'updated_at' => '2026-01-15T00:00:00.000Z'], 1500);
        $this->row($a, 'yellowmouse', ['toonId' => 'yellowmouse', 'variables' => ['tips' => 7], 'progress' => [], 'history' => ['event:y' => ['n' => 2, 'first' => 3, 'last' => 4]],
            'created_at' => '2026-03-01T00:00:00.000Z', 'updated_at' => '2026-03-01T00:00:00.000Z'], 3000);
        $this->row($a, 'scutoons', ['map' => ['places' => ['harbour' => ['unlocked' => 5]], 'routes' => (object) [], 'positions' => (object) []], 'created_at' => '2026-01-05T00:00:00.000Z'], 2500);
        // b: no save game yet, one toon
        $this->row($b, 'catpope', ['toonId' => 'catpope', 'variables' => ['k' => 'v'], 'progress' => [self::cp('catpope/chapter/1', '2026-01-01T00:00:00.000Z')], 'created_at' => '2026-01-01T00:00:00.000Z'], 100);
        // c: only a save game – untouched
        $this->row($c, 'scutoons', ['map' => ['places' => (object) []]], 50);
        $before = $this->db->value("SELECT state FROM progress WHERE user_id = ? AND toon_id = 'scutoons'", [$c]);

        $runner = new MigrationRunner($this->db, self::root() . '/migrations');
        $runner->runFile(self::root() . '/migrations/mysql/2026-10-06-global-save-game.mysql.sql');
        $first = $this->db->all("SELECT user_id, state, updated_at FROM progress WHERE toon_id = 'scutoons' ORDER BY user_id");
        $runner->runFile(self::root() . '/migrations/mysql/2026-10-06-global-save-game.mysql.sql');
        self::assertEquals($first, $this->db->all("SELECT user_id, state, updated_at FROM progress WHERE toon_id = 'scutoons' ORDER BY user_id"), 'idempotent');

        $s = $this->stored($a);
        ksort($s['variables']);
        self::assertSame(['catpope/tips' => 2, 'catpope/visits_harbour' => 3, 'toptoy/tips' => 5, 'toptoy/visits_hypercity' => 1, 'yellowmouse/tips' => 7], $s['variables'], 'each toon keeps its own tips');
        self::assertEqualsCanonicalizing(['catpope/chapter/2', 'toptoy/chapter/1'], array_column($s['progress'], 'uri'));
        self::assertEqualsCanonicalizing(['event:c', 'event:y'], array_keys($s['history']));
        self::assertSame(['harbour'], array_keys($s['map']['places']), 'the map stays');
        self::assertSame('2026-01-01T00:00:00.000Z', $s['created_at'], 'the earliest start');
        self::assertSame(3000, (int) $this->db->value("SELECT updated_at FROM progress WHERE user_id = ? AND toon_id = 'scutoons'", [$a]));
        self::assertSame(4, (int) $this->db->value('SELECT COUNT(*) FROM progress WHERE user_id = ?', [$a]), 'the per-toon rows stay');

        $s = $this->stored($b);
        self::assertSame(['catpope/k' => 'v'], $s['variables']);
        self::assertSame(['catpope/chapter/1'], array_column($s['progress'], 'uri'));
        self::assertSame($before, $this->db->value("SELECT state FROM progress WHERE user_id = ? AND toon_id = 'scutoons'", [$c]), 'no per-toon states: untouched');

        // the new viewer writes a newer save game; running the migration again keeps its values
        $this->get(ProgressService::class)->put($a, 'scutoons', self::obj(['variables' => ['catpope/tips' => 9]] + $this->stored($a)), 9000);
        $runner->runFile(self::root() . '/migrations/mysql/2026-10-06-global-save-game.mysql.sql');
        self::assertSame(9, $this->stored($a)['variables']['catpope/tips']);
    }
}
