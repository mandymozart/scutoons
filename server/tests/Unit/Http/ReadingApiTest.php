<?php
declare(strict_types=1);

namespace Scutoons\Tests\Http;

use Psr\Http\Message\ResponseInterface;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Domain\Progress\ReadingReport;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Kernel;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

/** Where readers are (BACKLOG 10.69): the admin user detail's `reading` and GET /api/admin/reading (per chapter) */
final class ReadingApiTest extends TestCase
{
    /** @param array<string, string> $headers */
    private function request(string $method, string $uri, array $headers = []): ResponseInterface
    {
        $request = (new ServerRequestFactory())->createServerRequest($method, "http://localhost$uri", ['REMOTE_ADDR' => '127.0.0.1']);
        foreach ($headers as $name => $value) $request = $request->withHeader($name, $value);
        $container = Kernel::container(Settings::fixed(self::root(), $this->settings()));
        $container->set(Clock::class, $this->clock);
        return Kernel::app($container)->handle($request);
    }

    /** @return array<string, mixed> */
    private static function json(ResponseInterface $response): array
    {
        return (array) json_decode((string) $response->getBody(), true);
    }

    /** @return array<string, string> */
    private function admin(): array
    {
        $user = $this->user('boss@example.com');
        $this->get(Roles::class)->grant((string) $user['id'], Roles::ADMIN);
        return ['Authorization' => 'Bearer ' . $this->get(Sessions::class)->create((string) $user['id'])];
    }

    /** @param list<array<string, mixed>> $checkpoints */
    private function saveGame(string $userId, array $checkpoints): void
    {
        $state = ['variables' => ['catpope/coins' => 3, 'tips' => true], 'progress' => $checkpoints, 'history' => [], 'map' => ['places' => ['harbour' => ['unlocked' => 1]]]];
        $this->db->run('INSERT INTO progress (user_id, toon_id, state, updated_at) VALUES (?, ?, ?, ?)', [$userId, 'scutoons', (string) json_encode($state, JSON_UNESCAPED_SLASHES), 1767225600000]);
    }

    /** @return array<string, mixed> */
    private static function cp(string $uri, string $at, bool $done, ?float $position, ?int $group = null, ?int $slice = null): array
    {
        $cp = ['uri' => $uri, 'isComplete' => $done, 'created_at' => '2026-01-01T00:00:00.000Z', 'updated_at' => $at];
        if ($position !== null) $cp['position'] = $position;
        if ($group !== null) $cp += ['group' => $group, 'slice' => $slice];
        return $cp;
    }

    public function testTheEndpointNeedsTheAdminRole(): void
    {
        $member = ['Authorization' => 'Bearer ' . $this->get(Sessions::class)->create((string) $this->user()['id'])];
        self::assertSame(401, $this->request('GET', '/api/admin/reading')->getStatusCode());
        self::assertSame(403, $this->request('GET', '/api/admin/reading', $member)->getStatusCode());
    }

    public function testOneMembersChaptersLastReadFirst(): void
    {
        $auth = $this->admin();
        $id = (string) $this->user()['id'];
        $none = self::json($this->request('GET', "/api/admin/users/$id", $auth));
        self::assertNull($none['user']['reading'], 'no save game on the server');

        $this->saveGame($id, [
            self::cp('catpope/chapter/1', '2026-01-02T00:00:00.000Z', true, 1),
            self::cp('catpope/chapter/2', '2026-01-03T00:00:00.000Z', false, 0.4321, 3, 5),
            self::cp('toptoy/chapter/1', '2026-01-01T12:00:00.000Z', true, null),
            ['uri' => 'catpope/somewhere/else', 'isComplete' => true],
        ]);
        $reading = self::json($this->request('GET', "/api/admin/users/$id", $auth))['user']['reading'];
        self::assertSame(['catpope/2', 'catpope/1', 'toptoy/1'], array_map(fn(array $c) => "{$c['toon']}/{$c['chapter']}", $reading['chapters']), 'chapters only, last read first');
        self::assertSame([0.432, false, 3, 5], [$reading['chapters'][0]['position'], $reading['chapters'][0]['finished'], $reading['chapters'][0]['group'], $reading['chapters'][0]['slice']]);
        self::assertSame([1, true, null], [$reading['chapters'][2]['position'], $reading['chapters'][2]['finished'], $reading['chapters'][2]['group']], 'an old checkpoint without position: finished = 1');
        self::assertSame([2, 1, 0], [$reading['variables'], $reading['places'], $reading['history']]);
        self::assertSame('2026-01-01T00:00:00+00:00', $reading['updated_at']);
    }

    public function testPerChapterStartedFinishedAndWhereTheyStopped(): void
    {
        $auth = $this->admin();
        $this->saveGame((string) $this->user()['id'], [
            self::cp('catpope/chapter/1', '2026-01-02T00:00:00.000Z', true, 0.2),
            self::cp('catpope/chapter/2', '2026-01-03T00:00:00.000Z', false, 0.45),
        ]);
        $this->saveGame((string) $this->user()['id'], [
            self::cp('catpope/chapter/1', '2026-01-05T00:00:00.000Z', false, 0.99),
            self::cp('toptoy/chapter/1', '2026-01-04T00:00:00.000Z', false, 0),
        ]);
        $this->saveGame((string) $this->user()['id'], []);
        // an older per-toon state is not the save game: not counted
        $this->db->run('INSERT INTO progress (user_id, toon_id, state, updated_at) VALUES (?, ?, ?, ?)', [(string) $this->user()['id'], 'catpope', '{"progress":[{"uri":"catpope/chapter/1","isComplete":true}]}', 1]);

        $body = self::json($this->request('GET', '/api/admin/reading', $auth));
        self::assertSame(3, $body['readers']);
        $byKey = array_column(array_map(fn(array $c) => $c + ['key' => "{$c['toon']}/{$c['chapter']}"], $body['chapters']), null, 'key');
        self::assertSame(['catpope/1', 'catpope/2', 'toptoy/1'], array_keys($byKey));
        $one = $byKey['catpope/1'];
        self::assertSame([2, 1, 1], [$one['started'], $one['finished'], $one['reading']]);
        self::assertSame([0, 0, 0, 0, 0, 0, 0, 0, 0, 1], $one['stops'], 'the finished reader is not in the stops; 0.99 is the last step');
        self::assertSame('2026-01-05T00:00:00.000Z', $one['last_read_at']);
        self::assertSame(1, $byKey['catpope/2']['stops'][4]);
        self::assertSame(1, $byKey['toptoy/1']['stops'][0]);

        $toptoy = self::json($this->request('GET', '/api/admin/reading?toon=toptoy', $auth));
        self::assertSame(['toptoy'], array_column($toptoy['chapters'], 'toon'));
        self::assertSame(400, $this->request('GET', '/api/admin/reading?toon=no%20toon', $auth)->getStatusCode());
        self::assertSame(ReadingReport::STEPS, count($one['stops']));
    }
}
