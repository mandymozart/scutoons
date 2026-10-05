<?php
declare(strict_types=1);

namespace Scutoons\Tests\Http;

use Psr\Http\Message\ResponseInterface;
use Scutoons\Domain\Agent\AgentChats;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Kernel;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

/** /api/agent-chats/* – the creator's Ask panel chat history (content editors, each only their own chats) */
final class AgentChatsApiTest extends TestCase
{
    private const A = '0b6f1c1e-2a3b-4c5d-8e9f-0a1b2c3d4e5f';
    private const B = '7d1e2f30-4a5b-4c6d-9e8f-1a2b3c4d5e6f';

    /** @param array<string, string> $headers */
    private function request(string $method, string $uri, array $headers = [], ?string $body = null): ResponseInterface
    {
        $request = (new ServerRequestFactory())->createServerRequest($method, "http://localhost$uri", ['REMOTE_ADDR' => '127.0.0.1']);
        foreach ($headers as $name => $value) $request = $request->withHeader($name, $value);
        if ($body !== null) {
            $request = $request->withHeader('Content-Type', 'application/json');
            $request->getBody()->write($body);
            $request->getBody()->rewind();
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

    /** @return array<string, string> */
    private function as(string $role, string $email = ''): array
    {
        $user = $this->user($email !== '' ? $email : $role . '@example.com');
        if ($role !== 'member') $this->get(Roles::class)->grant((string) $user['id'], $role);
        return ['Authorization' => 'Bearer ' . $this->get(Sessions::class)->create((string) $user['id'])];
    }

    /** @param array<string, mixed> $fields */
    private static function body(array $fields = []): string
    {
        return (string) json_encode($fields + [
            'title' => 'Find the screenplay',
            'page' => '/toons/catpope/script',
            'turns' => [['role' => 'user', 'text' => 'Where is "Not My City"?'], ['role' => 'assistant', 'text' => 'Chapter 3.']],
            'messages' => [['role' => 'user', 'content' => [['type' => 'tool_use', 'input' => new \stdClass(), 'score' => 1.0]]]],
        ], JSON_PRESERVE_ZERO_FRACTION);
    }

    public function testRoles(): void
    {
        self::assertSame(401, $this->request('GET', '/api/agent-chats')->getStatusCode());
        self::assertSame(401, $this->request('PUT', '/api/agent-chats/' . self::A, [], self::body())->getStatusCode());
        $member = $this->as('member');
        foreach ([['GET', '/api/agent-chats'], ['GET', '/api/agent-chats/' . self::A], ['PUT', '/api/agent-chats/' . self::A], ['DELETE', '/api/agent-chats/' . self::A]] as [$method, $path]) {
            $response = $this->request($method, $path, $member, $method === 'PUT' ? self::body() : null);
            self::assertSame(403, $response->getStatusCode(), "$method $path");
            self::assertSame(['error', 'forbidden', 'creator'], [self::json($response)['status'], self::json($response)['error'], self::json($response)['role']]);
        }
        self::assertSame(0, (int) $this->db->value('SELECT COUNT(*) FROM agent_chats'));
    }

    public function testCreateListGetReplaceDelete(): void
    {
        $creator = $this->as('creator');
        $created = $this->request('PUT', '/api/agent-chats/' . self::A, $creator, self::body());
        self::assertSame(200, $created->getStatusCode(), (string) $created->getBody());
        self::assertSame(['status' => 'success', 'chat' => ['id' => self::A, 'updated_at' => '2026-09-21T14:13:20+00:00']], self::json($created));

        $this->clock->advance(60);
        $this->request('PUT', '/api/agent-chats/' . self::B . '/', $creator, self::body(['title' => '  Second  ', 'page' => null])); // trailing slash works too

        $list = self::json($this->request('GET', '/api/agent-chats', $creator));
        self::assertSame('success', $list['status']);
        self::assertSame([self::B, self::A], array_column($list['chats'], 'id')); // newest updated first
        self::assertSame(['id' => self::A, 'title' => 'Find the screenplay', 'page' => '/toons/catpope/script', 'app' => 'creator', 'created_at' => '2026-09-21T14:13:20+00:00', 'updated_at' => '2026-09-21T14:13:20+00:00', 'turns' => 2], $list['chats'][1]);
        self::assertSame(['Second', null], [$list['chats'][0]['title'], $list['chats'][0]['page']]);
        self::assertSame([self::B], array_column(self::json($this->request('GET', '/api/agent-chats?limit=1', $creator))['chats'], 'id'));
        self::assertCount(1, self::json($this->request('GET', '/api/agent-chats?limit=0', $creator))['chats']); // clamped to 1..200

        $got = $this->request('GET', '/api/agent-chats/' . self::A, $creator);
        self::assertSame(200, $got->getStatusCode());
        $chat = self::json($got)['chat'];
        self::assertSame('Chapter 3.', $chat['turns'][1]['text']);
        self::assertArrayNotHasKey('messages', $chat);
        $raw = (string) $this->request('GET', '/api/agent-chats/' . strtoupper(self::A) . '?messages=1', $creator)->getBody();
        self::assertStringContainsString('"input":{},"score":1.0', $raw); // objects stay objects, floats stay floats
        self::assertSame('tool_use', self::json($this->request('GET', '/api/agent-chats/' . self::A . '?messages=1', $creator))['chat']['messages'][0]['content'][0]['type']);

        // replace: content and updated_at change, created_at stays; title / page left out keep the stored values
        $this->clock->advance(60);
        $replaced = $this->request('PUT', '/api/agent-chats/' . self::A, $creator, (string) json_encode(['turns' => [['role' => 'user', 'text' => 'again']], 'messages' => []]));
        self::assertSame(200, $replaced->getStatusCode(), (string) $replaced->getBody());
        $chat = self::json($this->request('GET', '/api/agent-chats/' . self::A . '?messages=1', $creator))['chat'];
        self::assertSame(['Find the screenplay', '/toons/catpope/script', '2026-09-21T14:13:20+00:00', '2026-09-21T14:15:20+00:00', [['role' => 'user', 'text' => 'again']], []],
            [$chat['title'], $chat['page'], $chat['created_at'], $chat['updated_at'], $chat['turns'], $chat['messages']]);
        self::assertSame([self::A, self::B], array_column(self::json($this->request('GET', '/api/agent-chats', $creator))['chats'], 'id'));
        $this->request('PUT', '/api/agent-chats/' . self::A, $creator, self::body(['title' => str_repeat('é', 250), 'page' => '']));
        $chat = self::json($this->request('GET', '/api/agent-chats/' . self::A, $creator))['chat'];
        self::assertSame([200, null], [mb_strlen($chat['title']), $chat['page']]);

        $deleted = $this->request('DELETE', '/api/agent-chats/' . self::A, $creator);
        self::assertSame(['status' => 'success'], self::json($deleted));
        self::assertSame(404, $this->request('GET', '/api/agent-chats/' . self::A, $creator)->getStatusCode());
        self::assertSame(404, $this->request('DELETE', '/api/agent-chats/' . self::A, $creator)->getStatusCode());
        self::assertSame([self::B], array_column(self::json($this->request('GET', '/api/agent-chats', $creator))['chats'], 'id'));
    }

    public function testCreatorsOnlySeeAndChangeTheirOwnChats(): void
    {
        $alice = $this->as('creator', 'alice@example.com');
        $bob = $this->as('creator', 'bob@example.com');
        $admin = $this->as('admin'); // a content editor too (Roles::CONTENT_EDITORS) – still only their own chats
        $this->request('PUT', '/api/agent-chats/' . self::A, $alice, self::body(['title' => 'Alice']));

        self::assertSame([], self::json($this->request('GET', '/api/agent-chats', $bob))['chats']);
        self::assertSame([], self::json($this->request('GET', '/api/agent-chats', $admin))['chats']);
        foreach ([$bob, $admin] as $other) {
            $get = $this->request('GET', '/api/agent-chats/' . self::A, $other);
            self::assertSame([404, 'not_found'], [$get->getStatusCode(), self::json($get)['error']]);
            self::assertSame(404, $this->request('GET', '/api/agent-chats/' . self::A . '?messages=1', $other)->getStatusCode());
            self::assertSame(404, $this->request('PUT', '/api/agent-chats/' . self::A, $other, self::body(['title' => 'Mine now']))->getStatusCode());
            self::assertSame(404, $this->request('DELETE', '/api/agent-chats/' . self::A, $other)->getStatusCode());
        }
        $chat = self::json($this->request('GET', '/api/agent-chats/' . self::A, $alice))['chat'];
        self::assertSame('Alice', $chat['title']); // never overwritten
        self::assertSame(1, (int) $this->db->value('SELECT COUNT(*) FROM agent_chats'));
    }

    public function testInvalidIdsAndBodies(): void
    {
        $creator = $this->as('creator');
        foreach (['GET', 'PUT', 'DELETE'] as $method) {
            $response = $this->request($method, '/api/agent-chats/not-a-uuid', $creator, $method === 'PUT' ? self::body() : null);
            self::assertSame([422, 'invalid_id'], [$response->getStatusCode(), self::json($response)['error']], $method);
        }
        self::assertSame(422, $this->request('GET', '/api/agent-chats/' . self::A . 'x', $creator)->getStatusCode());
        self::assertSame(404, $this->request('GET', '/api/agent-chats/' . self::B, $creator)->getStatusCode()); // well-formed, unknown

        $url = '/api/agent-chats/' . self::A;
        foreach ([
            ['', 'invalid_body'],
            ['[]', 'invalid_body'],
            ['{nope', 'invalid_body'],
            ['{"messages":[]}', 'invalid_turns'],
            ['{"turns":{},"messages":[]}', 'invalid_turns'],
            ['{"turns":"[]","messages":[]}', 'invalid_turns'],
            ['{"turns":[],"messages":{"a":1}}', 'invalid_messages'],
            ['{"turns":[]}', 'invalid_messages'],
            ['{"turns":[],"messages":[],"title":5}', 'invalid_title'],
            ['{"turns":[],"messages":[],"page":[]}', 'invalid_page'],
        ] as [$body, $error]) {
            $response = $this->request('PUT', $url, $creator, $body);
            self::assertSame([422, $error], [$response->getStatusCode(), self::json($response)['error'] ?? null], $body);
        }

        $big = (string) json_encode(['turns' => [], 'messages' => [str_repeat('x', AgentChats::MAX_BYTES)]]);
        $response = $this->request('PUT', $url, $creator, $big);
        self::assertSame([413, 'too_large'], [$response->getStatusCode(), self::json($response)['error']]);
        self::assertSame(0, (int) $this->db->value('SELECT COUNT(*) FROM agent_chats'));

        // just under the limit is stored (a long model conversation)
        $large = (string) json_encode(['turns' => [], 'messages' => [str_repeat('y', 3 * 1048576)]]);
        self::assertSame(200, $this->request('PUT', $url, $creator, $large)->getStatusCode());
    }

    public function testChatsPerAppWithTheRoleOfThatApp(): void
    {
        $liaison = $this->as('liaison');
        $creator = $this->as('creator');
        $admin = $this->as('admin', 'boss@example.com'); // creator + embassy + admin panels

        // the Embassy panel: the Embassy roles (liaison / ambassador / admin)
        $put = $this->request('PUT', '/api/agent-chats/' . self::A, $liaison, self::body(['app' => 'embassy', 'page' => '/orgs/x']));
        self::assertSame(200, $put->getStatusCode(), (string) $put->getBody());
        $list = self::json($this->request('GET', '/api/agent-chats?app=embassy', $liaison));
        self::assertSame([[self::A, 'embassy']], array_map(fn(array $c) => [$c['id'], $c['app']], $list['chats']));
        $chat = self::json($this->request('GET', '/api/agent-chats/' . self::A, $liaison))['chat']; // by id: the stored chat's app
        self::assertSame(['embassy', 2], [$chat['app'], count($chat['turns'])]);
        $forbidden = $this->request('GET', '/api/agent-chats', $liaison); // the creator panel needs a content editor
        self::assertSame([403, 'creator'], [$forbidden->getStatusCode(), self::json($forbidden)['role']]);
        $forbidden = $this->request('PUT', '/api/agent-chats/' . self::B, $creator, self::body(['app' => 'embassy']));
        self::assertSame([403, 'liaison'], [$forbidden->getStatusCode(), self::json($forbidden)['role']]);
        $forbidden = $this->request('PUT', '/api/agent-chats/' . self::B, $creator, self::body(['app' => 'admin']));
        self::assertSame([403, 'admin'], [$forbidden->getStatusCode(), self::json($forbidden)['role']]);
        $invalid = $this->request('PUT', '/api/agent-chats/' . self::B, $creator, self::body(['app' => 'shop']));
        self::assertSame([422, 'invalid_app'], [$invalid->getStatusCode(), self::json($invalid)['error']]);
        self::assertSame(422, $this->request('GET', '/api/agent-chats?app=shop', $creator)->getStatusCode());

        // one user, several panels: each list only has its app's chats; another app's id is 404, never overwritten
        $this->request('PUT', '/api/agent-chats/' . self::B, $admin, self::body(['app' => 'admin', 'title' => 'Admin chat']));
        self::assertSame([], self::json($this->request('GET', '/api/agent-chats', $admin))['chats']);
        self::assertSame([], self::json($this->request('GET', '/api/agent-chats?app=embassy', $admin))['chats']);
        self::assertSame([self::B], array_column(self::json($this->request('GET', '/api/agent-chats?app=admin', $admin))['chats'], 'id'));
        self::assertSame(404, $this->request('PUT', '/api/agent-chats/' . self::B, $admin, self::body(['title' => 'Now creator']))->getStatusCode());
        self::assertSame(404, $this->request('PUT', '/api/agent-chats/' . self::B, $admin, self::body(['app' => 'embassy']))->getStatusCode());
        self::assertSame(404, $this->request('GET', '/api/agent-chats/' . self::B . '?app=creator', $admin)->getStatusCode());
        self::assertSame(404, $this->request('DELETE', '/api/agent-chats/' . self::B . '?app=embassy', $admin)->getStatusCode());
        self::assertSame('Admin chat', self::json($this->request('GET', '/api/agent-chats/' . self::B . '?app=admin', $admin))['chat']['title']);
        self::assertSame(404, $this->request('GET', '/api/agent-chats/' . self::A, $admin)->getStatusCode()); // the liaison's chat
        self::assertSame(200, $this->request('DELETE', '/api/agent-chats/' . self::B, $admin)->getStatusCode());
        self::assertSame(['embassy'], array_column($this->db->all('SELECT app FROM agent_chats'), 'app'));
    }

    public function testWithoutTheMigrationTheEndpointsAnswer503(): void
    {
        $creator = $this->as('creator');
        $this->db->exec('DROP TABLE agent_chats');
        $response = $this->request('GET', '/api/agent-chats', $creator);
        self::assertSame([503, 'not_available'], [$response->getStatusCode(), self::json($response)['error']]);
        self::assertSame(503, $this->request('PUT', '/api/agent-chats/' . self::A, $creator, self::body())->getStatusCode());
    }
}
