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
 * The developer role and migrations through the API (BACKLOG 10.61): only a developer hands the role on, the last one
 * stays; a developer's AI connection plans (a dry run) and applies – the planned files exactly, the changed rows backed
 * up; the deploy runs them with MIGRATE_KEY. Only a failed run reaches the PHP error log (#166: the admin's Server
 * errors page shows every app line as an error); an applied run is its dev_migration_runs row alone.
 */
final class DevMigrationsApiTest extends TestCase
{
    private string $migrations;

    protected function setUp(): void
    {
        parent::setUp();
        $this->migrations = $this->dir . '/migrations';
        mkdir($this->migrations . '/mysql', 0777, true);
    }

    protected function tearDown(): void
    {
        $this->db->exec('ALTER TABLE users DROP COLUMN IF EXISTS dev_test_flag'); // the test database keeps its schema
        parent::tearDown();
    }

    protected function settings(): array
    {
        return ['MIGRATIONS_PATH' => $this->dir . '/migrations', 'MIGRATION_BACKUP_PATH' => $this->dir . '/backups', 'MIGRATE_KEY' => str_repeat('k', 40), 'LOG_PATH' => $this->dir . '/logs/error.log'] + parent::settings();
    }

    /** what the API wrote to the PHP error log (LOG_PATH) so far */
    private function errorLog(): string
    {
        $path = $this->dir . '/logs/error.log';
        return is_file($path) ? (string) file_get_contents($path) : '';
    }

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

    /**
     * @param list<string> $roles
     * @return array{0: array<string, string>, 1: string} the session header and the user id
     */
    private function session(array $roles, string $email): array
    {
        $user = $this->user($email);
        foreach ($roles as $role) $this->get(Roles::class)->grant((string) $user['id'], $role);
        return [['Authorization' => 'Bearer ' . $this->get(Sessions::class)->create((string) $user['id'])], (string) $user['id']];
    }

    private function migration(string $name, string $sql): void
    {
        file_put_contents("{$this->migrations}/mysql/$name.mysql.sql", $sql);
    }

    public function testOnlyADeveloperGrantsTheRoleAndTheLastOneStays(): void
    {
        [$admin] = $this->session([Roles::ADMIN], 'admin@scutoons.test');
        [$developer, $developerId] = $this->session([Roles::ADMIN, Roles::DEVELOPER], 'dev@scutoons.test');
        $other = (string) $this->user('other@scutoons.test')['id'];

        $r = $this->request('PUT', "/api/admin/users/$other/roles/developer", $admin);
        self::assertSame([403, 'developer'], [$r->getStatusCode(), self::json($r)['role'] ?? null], (string) $r->getBody());
        self::assertSame(403, $this->request('DELETE', "/api/admin/users/$developerId/roles/developer", $admin)->getStatusCode());
        // the last developer keeps it
        $r = $this->request('DELETE', "/api/admin/users/$developerId/roles/developer", $developer);
        self::assertSame([409, 'last_developer'], [$r->getStatusCode(), self::json($r)['error'] ?? null], (string) $r->getBody());
        // a developer hands it on; then one may go
        self::assertSame(200, $this->request('PUT', "/api/admin/users/$other/roles/developer", $developer)->getStatusCode());
        self::assertSame(200, $this->request('DELETE', "/api/admin/users/$developerId/roles/developer", $developer)->getStatusCode());
        // an admin is no developer: /api/dev is closed
        self::assertSame(403, $this->request('GET', '/api/dev/migrations', $admin)->getStatusCode());
    }

    public function testADevelopersAiConnectionPlansAndApplies(): void
    {
        [$developer] = $this->session([Roles::DEVELOPER], 'dev@scutoons.test');
        [$admin] = $this->session([Roles::ADMIN], 'admin@scutoons.test');
        $token = fn(array $session) => ['Authorization' => 'Bearer ' . self::json($this->request('POST', '/api/me/tokens', $session, ['name' => 'Claude Code']))['token']];
        $this->get(Roles::class)->grant((string) $this->db->value("SELECT id FROM users WHERE email = 'dev@scutoons.test'"), Roles::CREATOR); // a member role: tokens are for members
        $ai = $token($developer);
        self::assertSame(403, $this->request('GET', '/api/dev/migrations', $token($admin))->getStatusCode());

        $this->migration('2099-01-01-a', "UPDATE users SET display_name = 'Dev' WHERE email = 'dev@scutoons.test';
");
        self::assertSame(1, self::json($this->request('GET', '/api/dev/migrations', $ai))['pending']);
        $r = $this->request('POST', '/api/dev/migrations/plan', $ai);
        self::assertSame(201, $r->getStatusCode(), (string) $r->getBody());
        $plan = self::json($r)['run'];
        self::assertSame(['planned', 'ai:Claude Code', 1], [$plan['status'], $plan['created_via'], $plan['plan'][0]['statements'][0]['rows']]);
        $r = $this->request('POST', "/api/dev/migrations/runs/{$plan['id']}/apply", $ai);
        self::assertSame([200, 'applied'], [$r->getStatusCode(), self::json($r)['run']['status']], (string) $r->getBody());
        self::assertSame('Dev', $this->db->value("SELECT display_name FROM users WHERE email = 'dev@scutoons.test'"));

        // plan + apply in one go; nothing pending → no run
        $this->migration('2099-01-02-b', "UPDATE users SET display_name = 'Dev 2' WHERE email = 'dev@scutoons.test';
");
        $run = self::json($this->request('POST', '/api/dev/migrations/run', $ai))['run'];
        self::assertSame(['applied', ['2099-01-02-b.mysql.sql']], [$run['status'], $run['files']]);
        self::assertNull(self::json($this->request('POST', '/api/dev/migrations/run', $ai))['run']);
    }

    public function testTheDeployRunsThemWithTheKey(): void
    {
        $this->user('a@scutoons.test');
        $this->migration('2099-03-01-deploy', "UPDATE users SET display_name = 'Deployed' WHERE email = 'a@scutoons.test';
");
        self::assertSame(403, $this->request('POST', '/api/deploy/migrate')->getStatusCode());
        self::assertSame(403, $this->request('POST', '/api/deploy/migrate', ['X-Migrate-Key' => 'wrong'])->getStatusCode());
        $r = $this->request('POST', '/api/deploy/migrate', ['X-Migrate-Key' => str_repeat('k', 40)]);
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
        $run = self::json($r)['run'];
        self::assertSame(['applied', 'deploy', null], [$run['status'], $run['created_via'], $run['created_by']]);
        self::assertSame('Deployed', $this->db->value("SELECT display_name FROM users WHERE email = 'a@scutoons.test'"));
        self::assertNull(self::json($this->request('POST', '/api/deploy/migrate', ['X-Migrate-Key' => str_repeat('k', 40)]))['run']);
        // an applied run is only its row – nothing in the error log (#166)
        self::assertStringNotContainsString('migrations', $this->errorLog());
        // a failing run → 500, so the deploy job fails; the failure is in the error log
        $this->migration('2099-03-02-broken', "UPDATE no_such_table SET x = 1;
");
        self::assertSame(500, $this->request('POST', '/api/deploy/migrate', ['X-Migrate-Key' => str_repeat('k', 40)])->getStatusCode());
        self::assertMatchesRegularExpression('/\[qr-api\] migrations \S+ failed by the deploy: .*no_such_table/', $this->errorLog());
    }

    public function testPlanIsADryRunAndApplyRunsExactlyThePlannedFiles(): void
    {
        [$developer] = $this->session([Roles::DEVELOPER], 'dev@scutoons.test');
        $this->user('a@scutoons.test');
        $this->user('b@scutoons.test');
        $this->migration('2099-01-01-names', "-- data only\nUPDATE users SET display_name = 'Renamed' WHERE email LIKE '%@scutoons.test' AND email <> 'dev@scutoons.test';\n");
        $this->migration('2099-01-02-column', "ALTER TABLE users ADD COLUMN IF NOT EXISTS dev_test_flag TINYINT NOT NULL DEFAULT 0;\nUPDATE users SET dev_test_flag = 1;\n");

        $plan = self::json($this->request('POST', '/api/dev/migrations/plan', $developer))['run'];
        [$names, $column] = $plan['plan'];
        self::assertSame(['data', true, 2, null], [$names['kind'], $names['dry_run'], $names['statements'][0]['rows'], $names['error']]);
        self::assertSame(['mixed', false, null], [$column['kind'], $column['dry_run'], $column['statements'][0]['rows']]);
        // the dry run changed nothing
        self::assertSame(0, (int) $this->db->value("SELECT COUNT(*) FROM users WHERE display_name = 'Renamed'"));

        // a changed file makes the plan stale
        $this->migration('2099-01-01-names', "UPDATE users SET display_name = 'Other' WHERE email = 'a@scutoons.test';\n");
        $r = $this->request('POST', "/api/dev/migrations/runs/{$plan['id']}/apply", $developer);
        self::assertSame(409, $r->getStatusCode());
        self::assertStringContainsString('plan_stale', (string) $r->getBody());
        self::assertSame(409, $this->request('POST', "/api/dev/migrations/runs/{$plan['id']}/apply", $developer)->getStatusCode());

        // plan again, apply
        $this->migration('2099-01-01-names', "UPDATE users SET display_name = 'Renamed' WHERE email LIKE '%@scutoons.test' AND email <> 'dev@scutoons.test';\n");
        $plan = self::json($this->request('POST', '/api/dev/migrations/plan', $developer))['run'];
        $r = $this->request('POST', "/api/dev/migrations/runs/{$plan['id']}/apply", $developer);
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
        $run = self::json($r)['run'];
        self::assertSame(['applied', null], [$run['status'], $run['error']], (string) $run['log']);
        self::assertSame(2, (int) $this->db->value("SELECT COUNT(*) FROM users WHERE display_name = 'Renamed'"));
        self::assertSame(3, (int) $this->db->value('SELECT SUM(dev_test_flag) FROM users'));
        self::assertStringContainsString('library cache emptied', (string) $run['log']);
        self::assertSame(0, self::json($this->request('GET', '/api/dev/migrations', $developer))['pending']);

        // the rows were backed up before they changed
        $backup = $this->dir . '/backups/' . basename((string) $run['backup']);
        $rows = array_map(fn(string $l) => json_decode($l, true), file($backup, FILE_IGNORE_NEW_LINES) ?: []);
        $names = array_values(array_filter($rows, fn(array $r) => $r['migration'] === '2099-01-01-names.mysql.sql'));
        self::assertSame(['users', null], [$names[0]['table'], $names[0]['row']['display_name']]);
        self::assertCount(2, $names);
        // an applied plan isn't applied again
        self::assertSame(409, $this->request('POST', "/api/dev/migrations/runs/{$plan['id']}/apply", $developer)->getStatusCode());
    }

    public function testAFailingDataFileIsRolledBackWhole(): void
    {
        [$developer] = $this->session([Roles::DEVELOPER], 'dev@scutoons.test');
        $this->migration('2099-02-01-broken', "UPDATE users SET display_name = 'Half';\nUPDATE no_such_table SET x = 1;\n");
        $plan = self::json($this->request('POST', '/api/dev/migrations/plan', $developer))['run'];
        self::assertNotNull($plan['plan'][0]['error']);
        $run = self::json($this->request('POST', "/api/dev/migrations/runs/{$plan['id']}/apply", $developer))['run'];
        self::assertSame('failed', $run['status']);
        self::assertSame(0, (int) $this->db->value("SELECT COUNT(*) FROM users WHERE display_name = 'Half'"));
        self::assertSame(1, self::json($this->request('GET', '/api/dev/migrations', $developer))['pending']);
        self::assertStringContainsString("[qr-api] migrations {$run['id']} failed by dev@scutoons.test: ", $this->errorLog());
    }
}
