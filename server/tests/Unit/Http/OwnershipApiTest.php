<?php
declare(strict_types=1);

namespace Scutoons\Tests\Http;

use Psr\Http\Message\ResponseInterface;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Domain\Codes\CodeService;
use Scutoons\Domain\Inventory\InventoryService;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Kernel;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

/** The HTTP side of ownership claims and gifts (agents/ITEMS.md "Ownership, claims, gifting"): routes, auth, raw file upload, admin download */
final class OwnershipApiTest extends TestCase
{
    /** @param array<string, string> $headers */
    private function request(string $method, string $uri, array $headers = [], ?string $body = null): ResponseInterface
    {
        $request = (new ServerRequestFactory())->createServerRequest($method, "http://localhost$uri", ['REMOTE_ADDR' => '127.0.0.1']);
        foreach ($headers as $name => $value) $request = $request->withHeader($name, $value);
        if ($body !== null) {
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

    /**
     * @param array<string, mixed> $user
     * @return array<string, string>
     */
    private function auth(array $user): array
    {
        return ['Authorization' => 'Bearer ' . $this->get(Sessions::class)->create((string) $user['id']), 'Content-Type' => 'application/json'];
    }

    public function testClaimFlowOverHttp(): void
    {
        $owner = $this->user();
        $code = $this->code('membership', ['edition_id' => '3_catpope_vinyl']);
        $this->get(CodeService::class)->claim($owner, $code, null, null);
        $claimant = $this->auth($this->user('claimant@example.com'));

        self::assertSame(401, $this->request('POST', '/api/claims/', [], '{}')->getStatusCode());
        $r = $this->request('POST', '/api/claims/', $claimant, (string) json_encode(['code' => $code, 'statement' => 'Bought it at the show in Vienna.']));
        self::assertSame(201, $r->getStatusCode(), (string) $r->getBody());
        $claim = self::json($r)['claim'];

        $png = "\x89PNG\r\n\x1A\n" . str_repeat("\0", 32);
        $r = $this->request('POST', "/api/claims/{$claim['id']}/files?name=receipt.png", ['Content-Type' => 'image/png'] + $claimant, $png);
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
        $file = self::json($r)['claim']['files'][0];
        self::assertSame('receipt.png', $file['name']);

        $mine = self::json($this->request('GET', '/api/claims/', $claimant));
        self::assertSame([$claim['id']], array_column($mine['items'], 'id'));

        // admin: 401 / 403, list with the open count, detail, the file, transfer
        self::assertSame(401, $this->request('GET', '/api/admin/claims')->getStatusCode());
        self::assertSame(403, $this->request('GET', '/api/admin/claims', $claimant)->getStatusCode());
        $adminUser = $this->user('boss@example.com');
        $this->get(Roles::class)->grant((string) $adminUser['id'], Roles::ADMIN);
        $admin = $this->auth($adminUser);
        $list = self::json($this->request('GET', '/api/admin/claims?status=open', $admin));
        self::assertSame([1, 1], [$list['pagination']['total'], $list['open']]);
        $detail = self::json($this->request('GET', "/api/admin/claims/{$claim['id']}", $admin));
        self::assertSame($owner['email'], $detail['current_owner']['email']);
        $download = $this->request('GET', "/api/admin/claims/{$claim['id']}/files/{$file['id']}", $admin);
        self::assertSame([200, 'image/png', 'nosniff', $png], [$download->getStatusCode(), $download->getHeaderLine('Content-Type'),
            $download->getHeaderLine('X-Content-Type-Options'), (string) $download->getBody()]);
        self::assertSame(400, $this->request('POST', "/api/admin/claims/{$claim['id']}/transfer", $admin, '{}')->getStatusCode());
        $r = $this->request('POST', "/api/admin/claims/{$claim['id']}/transfer", $admin, '{"note":"receipt ok"}');
        self::assertSame('transferred', self::json($r)['claim']['status'], (string) $r->getBody());

        // the code page: history + a direct transfer back
        $history = self::json($this->request('GET', "/api/admin/codes/$code/history", $admin));
        self::assertSame(['claim'], array_column($history['items'], 'kind'));
        $r = $this->request('POST', "/api/admin/codes/$code/transfer", $admin, (string) json_encode(['to' => $owner['email'], 'note' => 'mistake, moved back']));
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
        self::assertSame([$owner['email'], ['admin', 'claim']], [self::json($r)['to']['email'], array_column(self::json($r)['history'], 'kind')]);
        self::assertSame(404, $this->request('GET', '/api/admin/codes/nope/history', $admin)->getStatusCode());
    }

    public function testGiftAndReceivedNoticeOverHttp(): void
    {
        $giverUser = $this->user('giver@example.com');
        $this->user('friend@example.com');
        $this->get(InventoryService::class)->add((string) $giverUser['id'], 'cucumber', null, 2, 'story');
        $r = $this->request('POST', '/api/inventory/gift/', $this->auth($giverUser), '{"item_id":"cucumber","qty":2,"to":"friend@example.com"}');
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
        self::assertSame(['cucumber', 2], [self::json($r)['gift']['item_id'], self::json($r)['gift']['qty']]);
        self::assertNotContains('cucumber', array_column(self::json($r)['inventory'], 'item_id'));

        $friend = $this->auth((array) $this->db->one('SELECT * FROM users WHERE email = ?', ['friend@example.com']));
        $me = self::json($this->request('GET', '/api/me/', $friend));
        self::assertSame([['gift', 'cucumber', 2, 'giver@example.com']], array_map(fn($n) => [$n['kind'], $n['item_id'], $n['qty'], $n['from']], $me['received']));
        self::assertSame(1, self::json($this->request('POST', '/api/inventory/received/seen/', $friend, '{}'))['seen']);
        self::assertSame([], self::json($this->request('GET', '/api/me/', $friend))['received']);
    }
}
