<?php
declare(strict_types=1);

namespace Scutoons\Tests\Http;

use Psr\Http\Message\ResponseInterface;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Domain\Content\EntityStore;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Kernel;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

/** The History's reverts (#687): POST /api/entities/<key>/revert and /revert-to – always a new draft, never a rewrite */
final class EntityRevertApiTest extends TestCase
{
    /** @var array<string, string> */
    private array $auth = [];

    private function request(string $method, string $uri, mixed $body = null): ResponseInterface
    {
        $request = (new ServerRequestFactory())->createServerRequest($method, "http://localhost$uri", ['REMOTE_ADDR' => '127.0.0.1']);
        foreach ($this->auth as $name => $value) $request = $request->withHeader($name, $value);
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
    private function json(ResponseInterface $response, int $status): array
    {
        self::assertSame($status, $response->getStatusCode(), (string) $response->getBody());
        return json_decode((string) $response->getBody(), true);
    }

    private const A = ['title' => 'Cat Pope', 'date' => '2026-11-14T20:00:00', 'location' => 'Arena', 'city' => 'Vienna', 'category' => 'irl', 'toon' => ['catpope']];
    private const B = ['title' => 'Feilchen', 'date' => '2026-11-15T20:00:00', 'location' => 'Arena', 'city' => 'Vienna', 'category' => 'irl', 'toon' => ['catpope']];

    /** @return array{0: int, 1: int, 2: int} three saves of the events document: none, A, A + B */
    private function threeVersions(): array
    {
        $user = $this->user('creator@example.com');
        $this->get(Roles::class)->grant((string) $user['id'], Roles::CREATOR);
        $this->auth = ['Authorization' => 'Bearer ' . $this->get(Sessions::class)->create((string) $user['id'])];
        $ids = [];
        $base = null;
        foreach ([[], [self::A], [self::A, self::B]] as $list) {
            $base = $this->json($this->request('PUT', '/api/entities/events', ['content' => $list, 'base' => $base]), 200)['version']['id'];
            $ids[] = $base;
        }
        return [$ids[0], $ids[1], $ids[2]];
    }

    public function testRevertToIsANewDraftOfThatVersion(): void
    {
        [$v1, , $v3] = $this->threeVersions();
        $back = $this->json($this->request('POST', '/api/entities/events/revert-to', ['version' => $v1, 'base' => $v3]), 200);
        self::assertSame(["back to #$v1", 'draft', false], [$back['version']['message'], $back['version']['status'], $back['unchanged']]);
        $store = $this->get(EntityStore::class);
        self::assertSame('[]', $store->latest('events')['json'] ?? null);
        self::assertCount(4, $store->versions('events')['versions'], 'nothing is deleted');

        $stale = $this->json($this->request('POST', '/api/entities/events/revert-to', ['version' => $v1, 'base' => $v3]), 409);
        self::assertSame(['conflict', $back['version']['id']], [$stale['error'] ?? $stale['code'] ?? null, $stale['latest']['id']]);
        $this->json($this->request('POST', '/api/entities/events/revert-to', ['version' => $v1]), 400);
        $this->json($this->request('POST', '/api/entities/events/revert-to', ['version' => 999999, 'base' => $back['version']['id']]), 404);
    }

    public function testRevertStoresTheCoresDocumentCheckedLikeASave(): void
    {
        [$v1, $v2, $v3] = $this->threeVersions();
        $done = $this->json($this->request('POST', '/api/entities/events/revert', ['version' => $v2, 'content' => [self::B], 'base' => $v3]), 200);
        self::assertSame("revert #$v2", $done['version']['message']);
        self::assertSame('Feilchen', json_decode((string) ($this->get(EntityStore::class)->latest('events')['json'] ?? ''), true)[0]['title'] ?? null);

        $first = $this->json($this->request('POST', '/api/entities/events/revert', ['version' => $v1, 'content' => [], 'base' => $done['version']['id']]), 422);
        self::assertSame('nothing_before', $first['error'] ?? $first['code'] ?? null);
        $this->json($this->request('POST', '/api/entities/events/revert', ['version' => $v2, 'base' => $done['version']['id']]), 400);
        $this->json($this->request('POST', '/api/entities/events/revert', ['version' => $v2, 'content' => [['title' => 'x']], 'base' => $done['version']['id']]), 422);
    }
}
