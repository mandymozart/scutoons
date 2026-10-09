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
 * Drafts the member confirms in the browser (agents/MCP.md "Draft → link"): Embassy mail drafts (checked like a send,
 * never sent by saving, only the author sends, gone once sent) and admin proposals (stored, applied / dismissed once).
 */
final class McpDraftsApiTest extends TestCase
{
    protected function settings(): array
    {
        return ['EMBASSY_URL' => 'https://embassy.test', 'ADMIN_URL' => 'https://admin.test'] + parent::settings();
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

    /** @return array<string, string> */
    private function auth(string $role, string $email = ''): array
    {
        $user = $this->user($email);
        if ($role !== '') $this->get(Roles::class)->grant((string) $user['id'], $role);
        return ['Authorization' => 'Bearer ' . $this->get(Sessions::class)->create((string) $user['id'])];
    }

    public function testMailDraftIsCheckedStoredAndSentByItsAuthorOnly(): void
    {
        $liaison = $this->auth('liaison', 'liaison@scutoons.test');
        $other = $this->auth('liaison', 'other@scutoons.test');
        $org = self::json($this->request('POST', '/api/embassy/orgs', $liaison, ['name' => 'Lido', 'city' => 'Berlin', 'country' => 'DE',
            'people' => [['name' => 'Jana', 'email' => 'jana@lido.test'], ['name' => 'Quiet', 'email' => 'quiet@lido.test', 'do_not_contact' => true]]]));
        $id = $org['org']['id'];

        // checked like a send: a stranger's address, someone who opted out, no subject
        self::assertSame(400, $this->request('POST', "/api/embassy/orgs/$id/drafts", $liaison, ['to' => 'x@example.com', 'subject' => 'Hi', 'text' => 'x'])->getStatusCode());
        self::assertSame(409, $this->request('POST', "/api/embassy/orgs/$id/drafts", $liaison, ['to' => 'quiet@lido.test', 'subject' => 'Hi', 'text' => 'x'])->getStatusCode());
        self::assertSame(400, $this->request('POST', "/api/embassy/orgs/$id/drafts", $liaison, ['to' => 'jana@lido.test', 'subject' => ' ', 'text' => 'x'])->getStatusCode());

        $r = $this->request('POST', "/api/embassy/orgs/$id/drafts", $liaison, ['to' => 'jana@lido.test', 'track' => 'live', 'subject' => 'Soundtrack night?', 'text' => 'Hi Jana', 'source' => 'agent']);
        self::assertSame(201, $r->getStatusCode(), (string) $r->getBody());
        $draft = self::json($r)['draft'];
        self::assertSame("https://embassy.test/orgs/$id?draft={$draft['id']}", self::json($r)['url']);
        self::assertSame(['Lido', 'jana@lido.test', 'live', 'agent', 'liaison@scutoons.test'], [$draft['org'], $draft['to'], $draft['track'], $draft['source'], $draft['author']]);
        // saving sends nothing
        self::assertFileDoesNotExist($this->dir . '/mail.log');

        // the organisation page and the start page list it
        self::assertSame([$draft['id']], array_column(self::json($this->request('GET', "/api/embassy/orgs/$id", $other))['drafts'], 'id'));
        self::assertSame([$draft['id']], array_column(self::json($this->request('GET', '/api/embassy/drafts', $other))['items'], 'id'));

        // another liaison can't send it in the author's name, nor discard it
        $mail = ['to' => 'jana@lido.test', 'track' => 'live', 'subject' => 'Soundtrack night?', 'text' => 'Hi Jana (edited)', 'draft_id' => $draft['id']];
        self::assertSame(403, $this->request('POST', "/api/embassy/orgs/$id/mail", $other, $mail)->getStatusCode());
        self::assertSame(403, $this->request('DELETE', "/api/embassy/drafts/{$draft['id']}", $other)->getStatusCode());

        // the author sends it (edited) – the draft is gone
        $r = $this->request('POST', "/api/embassy/orgs/$id/mail", $liaison, $mail);
        self::assertSame(201, $r->getStatusCode(), (string) $r->getBody());
        self::assertStringContainsString('Hi Jana (edited)', (string) file_get_contents($this->dir . '/mail.log'));
        self::assertSame(404, $this->request('GET', "/api/embassy/drafts/{$draft['id']}", $liaison)->getStatusCode());
        self::assertSame([], self::json($this->request('GET', "/api/embassy/orgs/$id", $liaison))['drafts']);

        // an ambassador may discard anyone's draft
        $second = self::json($this->request('POST', "/api/embassy/orgs/$id/drafts", $liaison, ['to' => 'jana@lido.test', 'subject' => 'Again', 'text' => 'x']))['draft'];
        self::assertSame('person', $second['source']);
        self::assertSame(200, $this->request('DELETE', "/api/embassy/drafts/{$second['id']}", $this->auth('ambassador', 'amb@scutoons.test'))->getStatusCode());
        self::assertSame(404, $this->request('GET', "/api/embassy/drafts/{$second['id']}", $liaison)->getStatusCode());

        // readers have no Embassy
        self::assertSame(403, $this->request('POST', "/api/embassy/orgs/$id/drafts", $this->auth('', 'reader@scutoons.test'), ['to' => 'jana@lido.test', 'subject' => 'x', 'text' => 'x'])->getStatusCode());
    }

    public function testAdminProposalIsStoredAndDecidedOnce(): void
    {
        $admin = $this->auth(Roles::ADMIN, 'admin@scutoons.test');
        $changes = [['type' => 'catalog', 'kind' => 'items', 'id' => 'camera', 'label' => 'Camera', 'fields' => [['path' => 'name.en', 'before' => 'Camera', 'after' => 'Instant camera']]]];

        self::assertSame(403, $this->request('POST', '/api/admin/proposals', $this->auth('creator', 'c@scutoons.test'), ['title' => 'x', 'changes' => $changes])->getStatusCode());
        self::assertSame(400, $this->request('POST', '/api/admin/proposals', $admin, ['title' => 'x', 'changes' => []])->getStatusCode());
        self::assertSame(400, $this->request('POST', '/api/admin/proposals', $admin, ['title' => 'x', 'changes' => [['type' => 'codes']]])->getStatusCode());

        $r = $this->request('POST', '/api/admin/proposals', $admin, ['title' => 'Camera: new name', 'changes' => $changes, 'source' => 'agent']);
        self::assertSame(201, $r->getStatusCode(), (string) $r->getBody());
        $p = self::json($r)['proposal'];
        self::assertSame("https://admin.test/proposals/{$p['id']}", self::json($r)['url']);
        self::assertSame(['open', 'agent', $changes, 'admin@scutoons.test'], [$p['status'], $p['source'], $p['changes'], $p['author']]);

        $list = self::json($this->request('GET', '/api/admin/proposals?status=open', $admin))['items'];
        self::assertSame([[$p['id'], 1]], array_map(fn(array $i) => [$i['id'], $i['count']], $list));
        self::assertSame([], self::json($this->request('GET', '/api/admin/proposals?status=applied', $admin))['items']);

        $r = $this->request('PUT', "/api/admin/proposals/{$p['id']}", $admin, ['status' => 'applied']);
        self::assertSame(['applied', 'admin@scutoons.test'], [self::json($r)['proposal']['status'], self::json($r)['proposal']['decided_by']]);
        // once
        self::assertSame(409, $this->request('PUT', "/api/admin/proposals/{$p['id']}", $admin, ['status' => 'dismissed'])->getStatusCode());
        self::assertSame(400, $this->request('PUT', "/api/admin/proposals/{$p['id']}", $admin, ['status' => 'open'])->getStatusCode());
    }
}
