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

/** POST /api/agent-usage (the Ask service, creator session + service key) and GET /api/admin/agent-usage (admin) */
final class AgentUsageApiTest extends TestCase
{
    private const KEY = 'test-agent-service-key-0123456789abcdef';
    private const CHAT = '0b6f1c1e-2a3b-4c5d-8e9f-0a1b2c3d4e5f';

    private string $serviceKey = self::KEY;

    /** @return array<string, string> */
    protected function settings(): array
    {
        return ['AGENT_SERVICE_KEY' => $this->serviceKey] + parent::settings();
    }

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

    /** @return array{0: string, 1: array<string, string>} the user id and its Bearer header */
    private function as(string $role, string $email = ''): array
    {
        $user = $this->user($email !== '' ? $email : $role . '@example.com');
        if ($role !== 'member') $this->get(Roles::class)->grant((string) $user['id'], $role);
        return [(string) $user['id'], ['Authorization' => 'Bearer ' . $this->get(Sessions::class)->create((string) $user['id'])]];
    }

    /**
     * @param array<string, string> $session
     * @return array<string, string>
     */
    private static function service(array $session, string $key = self::KEY): array
    {
        return $session + ['X-Service-Key' => $key];
    }

    /** @param array<string, mixed> $fields */
    private static function body(array $fields = []): string
    {
        return (string) json_encode($fields + [
            'service' => 'anthropic',
            'model' => 'claude-opus-5-5',
            'chat_id' => self::CHAT,
            'input_tokens' => 1200,
            'output_tokens' => 300,
            'cache_read_tokens' => 5000,
            'cache_write_tokens' => 800,
            'cost_usd' => 0.0425,
        ]);
    }

    /**
     * @param array<string, string> $headers
     * @param array<string, mixed> $fields
     */
    private function log(array $headers, array $fields = []): ResponseInterface
    {
        return $this->request('POST', '/api/agent-usage', $headers, self::body($fields));
    }

    public function testWriteWithKeyAndCreatorSession(): void
    {
        [$id, $creator] = $this->as('creator');
        $other = $this->user('other@example.com');
        $response = $this->log(self::service($creator), ['user_id' => (string) $other['id'], 'chat_id' => strtoupper(self::CHAT)]);
        self::assertSame(201, $response->getStatusCode(), (string) $response->getBody());
        self::assertSame(['status' => 'success'], self::json($response));

        $row = $this->db->one('SELECT * FROM agent_usage');
        self::assertNotNull($row);
        self::assertSame([$id, '2026-09-21 14:13:20', 'anthropic', 'claude-opus-5-5', self::CHAT, 1200, 300, 5000, 800, 0.0425],
            [$row['user_id'], $row['created_at'], $row['service'], $row['model'], $row['chat_id'], (int) $row['input_tokens'], (int) $row['output_tokens'],
                (int) $row['cache_read_tokens'], (int) $row['cache_write_tokens'], round((float) $row['cost_usd'], 6)]);

        // chat_id is optional; strings are trimmed and cut; a Bearer request without an Origin passes the CSRF check
        $response = $this->request('POST', '/api/agent-usage', self::service($creator), (string) json_encode([
            'service' => '  ' . str_repeat('s', 50), 'model' => str_repeat('m', 120), 'input_tokens' => 0, 'output_tokens' => 0,
            'cache_read_tokens' => 0, 'cache_write_tokens' => 0, 'cost_usd' => 0,
        ]));
        self::assertSame(201, $response->getStatusCode(), (string) $response->getBody());
        $row = $this->db->one('SELECT * FROM agent_usage ORDER BY id DESC LIMIT 1');
        self::assertNotNull($row);
        self::assertSame([40, 100, null], [strlen((string) $row['service']), strlen((string) $row['model']), $row['chat_id']]);
    }

    public function testServiceKeyAndRoles(): void
    {
        [, $creator] = $this->as('creator');
        [, $member] = $this->as('member');

        $wrong = $this->log(self::service($creator, 'nope'));
        self::assertSame([403, 'forbidden'], [$wrong->getStatusCode(), self::json($wrong)['error']]);
        $missing = $this->log($creator);
        self::assertSame([403, 'forbidden'], [$missing->getStatusCode(), self::json($missing)['error']]);

        $anonymous = $this->log(['X-Service-Key' => self::KEY]);
        self::assertSame([401, 'unauthorized'], [$anonymous->getStatusCode(), self::json($anonymous)['error']]);
        $notCreator = $this->log(self::service($member));
        self::assertSame([403, 'forbidden', 'creator'], [$notCreator->getStatusCode(), self::json($notCreator)['error'], self::json($notCreator)['role']]);

        self::assertSame(0, (int) $this->db->value('SELECT COUNT(*) FROM agent_usage'));
    }

    public function testNotConfigured(): void
    {
        [, $creator] = $this->as('creator');
        $this->serviceKey = '';
        $response = $this->log(self::service($creator, ''));
        self::assertSame([503, 'not_configured'], [$response->getStatusCode(), self::json($response)['error']]);
        $response = $this->log(self::service($creator, 'anything'));
        self::assertSame([503, 'not_configured'], [$response->getStatusCode(), self::json($response)['error']]);
    }

    public function testValidation(): void
    {
        [, $creator] = $this->as('creator');
        $headers = self::service($creator);
        foreach ([
            ['', 'invalid_body'],
            ['[]', 'invalid_body'],
            ['{nope', 'invalid_body'],
        ] as [$body, $error]) {
            $response = $this->request('POST', '/api/agent-usage', $headers, $body);
            self::assertSame([422, $error], [$response->getStatusCode(), self::json($response)['error'] ?? null], $body);
        }
        foreach ([
            [['service' => null], 'invalid_service'],
            [['service' => '   '], 'invalid_service'],
            [['service' => 5], 'invalid_service'],
            [['model' => ''], 'invalid_model'],
            [['model' => ['x']], 'invalid_model'],
            [['chat_id' => 'not-a-uuid'], 'invalid_chat_id'],
            [['chat_id' => 12], 'invalid_chat_id'],
            [['input_tokens' => -1], 'invalid_input_tokens'],
            [['input_tokens' => null], 'invalid_input_tokens'],
            [['output_tokens' => 1.5], 'invalid_output_tokens'],
            [['cache_read_tokens' => '10'], 'invalid_cache_read_tokens'],
            [['cache_write_tokens' => 3000000000], 'invalid_cache_write_tokens'],
            [['cost_usd' => -0.01], 'invalid_cost_usd'],
            [['cost_usd' => 1000], 'invalid_cost_usd'],
            [['cost_usd' => '0.5'], 'invalid_cost_usd'],
            [['cost_usd' => null], 'invalid_cost_usd'],
        ] as [$fields, $error]) {
            $response = $this->log($headers, $fields);
            self::assertSame([422, $error], [$response->getStatusCode(), self::json($response)['error'] ?? null], (string) json_encode($fields));
        }
        self::assertSame(0, (int) $this->db->value('SELECT COUNT(*) FROM agent_usage'));
        self::assertSame(201, $this->log($headers, ['cost_usd' => 999.999999])->getStatusCode());
    }

    public function testAdminOverviewByDayAndMonth(): void
    {
        [$alice, $aliceSession] = $this->as('creator', 'alice@example.com');
        [$bob, $bobSession] = $this->as('creator', 'bob@example.com');
        [, $admin] = $this->as('admin');

        // 2026-08-31 (last month): alice anthropic
        $this->clock->advance(-21 * 86400);
        $this->log(self::service($aliceSession), ['cost_usd' => 0.5, 'input_tokens' => 10]);
        // 2026-09-21 (today): alice anthropic ×2, alice openai, bob anthropic
        $this->clock->advance(21 * 86400);
        $this->log(self::service($aliceSession), ['cost_usd' => 0.1, 'input_tokens' => 100]);
        $this->log(self::service($aliceSession), ['cost_usd' => 0.2, 'input_tokens' => 200]);
        $this->log(self::service($aliceSession), ['service' => 'openai', 'model' => 'gpt-x', 'cost_usd' => 0.05, 'input_tokens' => 1]);
        $this->log(self::service($bobSession), ['cost_usd' => 1.25, 'input_tokens' => 7, 'chat_id' => null]);
        // 2026-09-20: bob
        $this->clock->advance(-86400);
        $this->log(self::service($bobSession), ['cost_usd' => 0.01, 'input_tokens' => 3]);
        $this->clock->advance(86400);
        self::assertSame(6, (int) $this->db->value('SELECT COUNT(*) FROM agent_usage'));

        // default: the last 30 days by day
        $response = $this->request('GET', '/api/admin/agent-usage', $admin);
        self::assertSame(200, $response->getStatusCode(), (string) $response->getBody());
        $data = self::json($response);
        self::assertSame(['success', '2026-08-23', '2026-09-21', 'day'], [$data['status'], $data['from'], $data['to'], $data['group']]);
        self::assertSame([
            ['2026-09-21', $bob, 'bob@example.com', 'anthropic', 'claude-opus-5-5', 1, 1.25],
            ['2026-09-21', $alice, 'alice@example.com', 'anthropic', 'claude-opus-5-5', 2, 0.3],
            ['2026-09-21', $alice, 'alice@example.com', 'openai', 'gpt-x', 1, 0.05],
            ['2026-09-20', $bob, 'bob@example.com', 'anthropic', 'claude-opus-5-5', 1, 0.01],
            ['2026-08-31', $alice, 'alice@example.com', 'anthropic', 'claude-opus-5-5', 1, 0.5],
        ], array_map(fn(array $r) => [$r['period'], $r['user_id'], $r['email'], $r['service'], $r['model'], $r['calls'], $r['cost_usd']], $data['rows']));
        self::assertSame(['calls' => 2, 'input_tokens' => 300, 'output_tokens' => 600, 'cache_read_tokens' => 10000, 'cache_write_tokens' => 1600, 'cost_usd' => 0.3],
            array_diff_key($data['rows'][1], array_flip(['period', 'user_id', 'email', 'service', 'model'])));
        self::assertSame(['calls' => 6, 'input_tokens' => 321, 'output_tokens' => 1800, 'cache_read_tokens' => 30000, 'cache_write_tokens' => 4800, 'cost_usd' => 2.11], $data['totals']);

        // by month
        $data = self::json($this->request('GET', '/api/admin/agent-usage?from=2026-08-01&to=2026-09-30&group=month', $admin));
        self::assertSame(['2026-08-01', '2026-09-30', 'month'], [$data['from'], $data['to'], $data['group']]);
        self::assertSame([
            ['2026-09', 'bob@example.com', 'anthropic', 2, 1.26],
            ['2026-09', 'alice@example.com', 'anthropic', 2, 0.3],
            ['2026-09', 'alice@example.com', 'openai', 1, 0.05],
            ['2026-08', 'alice@example.com', 'anthropic', 1, 0.5],
        ], array_map(fn(array $r) => [$r['period'], $r['email'], $r['service'], $r['calls'], $r['cost_usd']], $data['rows']));

        // the range includes both days; outside it nothing
        $data = self::json($this->request('GET', '/api/admin/agent-usage?from=2026-09-20&to=2026-09-20', $admin));
        self::assertSame([['2026-09-20', 'bob@example.com']], array_map(fn(array $r) => [$r['period'], $r['email']], $data['rows']));
        $data = self::json($this->request('GET', '/api/admin/agent-usage?from=2025-01-01&to=2025-01-31', $admin));
        self::assertSame([[], ['calls' => 0, 'input_tokens' => 0, 'output_tokens' => 0, 'cache_read_tokens' => 0, 'cache_write_tokens' => 0, 'cost_usd' => 0.0]], [$data['rows'], $data['totals']]);
    }

    public function testAdminOverviewQueryErrorsAndRoles(): void
    {
        [, $admin] = $this->as('admin');
        [, $creator] = $this->as('creator');
        foreach ([
            ['?group=week', 'invalid_group'],
            ['?from=2026-13-01', 'invalid_from'],
            ['?from=2026-02-30', 'invalid_from'],
            ['?to=yesterday', 'invalid_to'],
            ['?from=2026-09-22&to=2026-09-21', 'invalid_range'],
            ['?from=2025-08-16&to=2026-09-21', 'invalid_range'], // 402 days
        ] as [$query, $error]) {
            $response = $this->request('GET', '/api/admin/agent-usage' . $query, $admin);
            self::assertSame([422, $error], [$response->getStatusCode(), self::json($response)['error'] ?? null], $query);
        }
        self::assertSame(200, $this->request('GET', '/api/admin/agent-usage?from=2025-08-18&to=2026-09-21', $admin)->getStatusCode()); // 400 days

        self::assertSame(401, $this->request('GET', '/api/admin/agent-usage')->getStatusCode());
        self::assertSame(403, $this->request('GET', '/api/admin/agent-usage', $creator)->getStatusCode());
    }

    public function testWithoutTheMigrationTheEndpointsAnswer503(): void
    {
        [, $creator] = $this->as('creator');
        [, $admin] = $this->as('admin');
        $this->db->exec('DROP TABLE agent_usage');
        $response = $this->log(self::service($creator));
        self::assertSame([503, 'not_available'], [$response->getStatusCode(), self::json($response)['error']]);
        $response = $this->request('GET', '/api/admin/agent-usage', $admin);
        self::assertSame([503, 'not_available'], [$response->getStatusCode(), self::json($response)['error']]);
    }
}
