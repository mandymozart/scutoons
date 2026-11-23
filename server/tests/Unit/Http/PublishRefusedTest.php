<?php
declare(strict_types=1);

namespace Scutoons\Tests\Http;

use Psr\Http\Message\ResponseInterface;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Kernel;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

/**
 * POST /api/entities/publish with an entry the catalog refuses: nothing goes live, and the answer names the entry by
 * its key, so the caller can leave it out and publish the rest (the creator's Publish, publish-together.ts)
 */
final class PublishRefusedTest extends TestCase
{
    /** @return array<string, mixed> */
    private static function carrier(string $id): array
    {
        return ['id' => $id, 'name' => ucfirst($id), 'image' => ''];
    }

    public function testTheRefusedEntryIsNamedAndTheRestPublishesWithoutIt(): void
    {
        $user = $this->user('writer@example.com');
        $this->get(Roles::class)->grant((string) $user['id'], Roles::CREATOR);
        foreach (['good', 'bad'] as $id) {
            self::assertSame(200, $this->request('PUT', "/api/entities/airlift-carrier:$id", $user, ['content' => self::carrier($id)])->getStatusCode());
        }
        // a stored version the catalog no longer takes (a draft is checked less than a publish: instruments, presets, rigs)
        $this->db->run("UPDATE entity_versions SET json = ? WHERE entity_key = 'airlift-carrier:bad'", [(string) json_encode(['name' => ''] + self::carrier('bad'))]);

        $both = $this->request('POST', '/api/entities/publish', $user, ['versions' => [['key' => 'airlift-carrier:good'], ['key' => 'airlift-carrier:bad']]]);
        $answer = (array) json_decode((string) $both->getBody(), true);
        self::assertSame([422, 'invalid_catalog', 'airlift-carrier:bad'], [$both->getStatusCode(), $answer['error'] ?? null, $answer['key'] ?? null], (string) $both->getBody());
        self::assertNotSame([], $answer['problems'] ?? []);
        self::assertSame(0, (int) $this->db->value("SELECT COUNT(*) FROM catalog_airlift_carriers WHERE id IN ('good', 'bad')"), 'all or nothing: the good one waits too');

        $rest = $this->request('POST', '/api/entities/publish', $user, ['versions' => [['key' => 'airlift-carrier:good']]]);
        self::assertSame(200, $rest->getStatusCode(), (string) $rest->getBody());
        self::assertSame(['good'], array_column($this->db->all("SELECT id FROM catalog_airlift_carriers WHERE id IN ('good', 'bad')"), 'id'));
    }

    /**
     * @param array<string, mixed> $user
     * @param array<string, mixed> $body
     */
    private function request(string $method, string $uri, array $user, array $body): ResponseInterface
    {
        $request = (new ServerRequestFactory())->createServerRequest($method, "http://localhost$uri", ['REMOTE_ADDR' => '127.0.0.1'])
            ->withHeader('Authorization', 'Bearer ' . $this->get(Sessions::class)->create((string) $user['id']))
            ->withHeader('Content-Type', 'application/json');
        $request->getBody()->write((string) json_encode($body));
        $request->getBody()->rewind();
        $container = Kernel::container(Settings::fixed(self::root(), $this->settings()));
        $container->set(Clock::class, $this->clock);
        return Kernel::app($container)->handle($request);
    }
}
