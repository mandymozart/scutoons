<?php
declare(strict_types=1);

namespace Scutoons\Tests\Http;

use Psr\Http\Message\ResponseInterface;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Domain\Layouts\DeviceSessions;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Kernel;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

/**
 * Resume across devices (Domain\Layouts\DeviceSessions): each device keeps its own state, another device sees the newest
 * other one; own rows only; only the shape and the size are checked; an AI connection only reads; before the migration
 * nothing throws.
 */
final class DeviceSessionApiTest extends TestCase
{
    private const STUDIO = 'studio-0001';
    private const LAPTOP = 'laptop-0002';
    private const STATE = ['v' => 1, 'window' => ['w' => 1920, 'h' => 1080], 'tabs' => ['tabs' => [['address' => 'toon:catpope']], 'front' => 'toon:catpope']];

    /** @param array<string, string> $headers */
    private function request(string $method, string $uri, array $headers = [], mixed $body = null): ResponseInterface
    {
        $request = (new ServerRequestFactory())->createServerRequest($method, "http://localhost$uri", ['REMOTE_ADDR' => '127.0.0.1']);
        $query = parse_url($uri, PHP_URL_QUERY);
        if (is_string($query)) {
            parse_str($query, $params);
            $request = $request->withQueryParams($params);
        }
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

    /** @param array<string, string> $who */
    private function put(array $who, string $device, mixed $state, string $name = 'Firefox · Windows'): ResponseInterface
    {
        return $this->request('PUT', '/api/me/session', $who, ['device' => $device, 'name' => $name, 'state' => $state]);
    }

    public function testEachDeviceKeepsItsOwnAndSeesTheNewestOther(): void
    {
        [$member] = $this->session('mira@scutoons.test');
        self::assertSame(401, $this->request('GET', '/api/me/session?device=' . self::STUDIO)->getStatusCode());
        $empty = self::json($this->request('GET', '/api/me/session?device=' . self::STUDIO, $member));
        self::assertSame([true, null, null], [$empty['available'], $empty['mine'], $empty['other']]);

        $r = $this->put($member, self::STUDIO, self::STATE, '  Firefox   · Windows ');
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
        $saved = self::json($r)['session'];
        self::assertSame([self::STUDIO, 'Firefox · Windows', self::STATE], [$saved['device'], $saved['name'], $saved['state']]);

        $this->clock->advance(60);
        $later = ['v' => 1, 'tabs' => ['tabs' => [], 'front' => null]];
        $this->put($member, self::LAPTOP, $later, 'Safari · macOS');

        // the studio sees its own and the laptop's, newer
        $studio = self::json($this->request('GET', '/api/me/session?device=' . self::STUDIO, $member));
        self::assertSame(self::STATE, $studio['mine']['state']);
        self::assertSame([self::LAPTOP, 'Safari · macOS', $later], [$studio['other']['device'], $studio['other']['name'], $studio['other']['state']]);
        self::assertGreaterThan((string) $studio['mine']['updated_at'], (string) $studio['other']['updated_at']);

        // saving again replaces the device's row: one per device
        $this->clock->advance(60);
        $this->put($member, self::STUDIO, $later);
        $laptop = self::json($this->request('GET', '/api/me/session?device=' . self::LAPTOP, $member));
        self::assertSame([self::STUDIO, $later], [$laptop['other']['device'], $laptop['other']['state']]);
        self::assertSame(2, (int) $this->db->value('SELECT COUNT(*) FROM member_sessions'));
    }

    public function testAnotherAppsRowIsAskedForAndNeverOfferedToTheCreator(): void
    {
        [$member] = $this->session('deck@scutoons.test');
        $this->put($member, self::STUDIO, self::STATE);
        $this->clock->advance(60);
        $deck = ['app' => 'deck', 'v' => 1, 'workspace' => 'stage', 'last' => ['stage' => 'scene:avatar-home']];
        $this->put($member, 'deck-x1-0003', $deck, 'Deck · X1');

        $laptop = self::json($this->request('GET', '/api/me/session?device=' . self::LAPTOP, $member));
        self::assertSame(self::STUDIO, $laptop['other']['device']);
        self::assertArrayNotHasKey('app', $laptop);
        $asked = self::json($this->request('GET', '/api/me/session?device=' . self::LAPTOP . '&app=deck', $member));
        self::assertSame([self::STUDIO, 'deck-x1-0003', $deck], [$asked['other']['device'], $asked['app']['device'], $asked['app']['state']]);
        self::assertNull(self::json($this->request('GET', '/api/me/session?device=' . self::LAPTOP . '&app=other', $member))['app']);
        self::assertSame('400 bad_app', self::error($this->request('GET', '/api/me/session?device=' . self::LAPTOP . '&app=Deck!', $member)));
    }

    public function testAMemberSeesOnlyTheirOwn(): void
    {
        [$mira] = $this->session('mira@scutoons.test');
        [$ben] = $this->session('ben@scutoons.test');
        $this->put($mira, self::STUDIO, self::STATE);
        $seen = self::json($this->request('GET', '/api/me/session?device=' . self::LAPTOP, $ben));
        self::assertSame([null, null], [$seen['mine'], $seen['other']]);
        // the same device id under another member is that member's own row
        $this->put($ben, self::STUDIO, ['v' => 1]);
        self::assertSame(self::STATE, self::json($this->request('GET', '/api/me/session?device=' . self::STUDIO, $mira))['mine']['state']);
    }

    public function testOnlyTheShapeAndTheSizeAreChecked(): void
    {
        [$member] = $this->session('shape@scutoons.test');
        self::assertSame('400 bad_device', self::error($this->request('GET', '/api/me/session', $member)));
        self::assertSame('400 bad_device', self::error($this->put($member, 'short', self::STATE)));
        self::assertSame('400 bad_device', self::error($this->put($member, 'not an id!', self::STATE)));
        self::assertSame('400 bad_state', self::error($this->put($member, self::STUDIO, [1, 2])));
        self::assertSame('400 bad_state', self::error($this->put($member, self::STUDIO, 'tabs')));
        self::assertSame('413 state_too_large', self::error($this->put($member, self::STUDIO, ['x' => str_repeat('a', DeviceSessions::MAX_BYTES)])));
        $r = $this->put($member, self::STUDIO, [], str_repeat('n', 100));
        self::assertSame(DeviceSessions::MAX_NAME, mb_strlen((string) self::json($r)['session']['name']));
    }

    public function testTheOldestDeviceGoes(): void
    {
        [$member, $id] = $this->session('many@scutoons.test');
        for ($i = 0; $i < DeviceSessions::MAX_DEVICES; $i++) {
            $this->put($member, sprintf('device-%04d', $i), ['v' => 1]);
            $this->clock->advance(1);
        }
        $this->put($member, 'device-new1', ['v' => 1]);
        $ids = array_map(static fn(array $r) => (string) $r['device_id'], $this->db->all('SELECT device_id FROM member_sessions WHERE user_id = ?', [$id]));
        self::assertCount(DeviceSessions::MAX_DEVICES, $ids);
        self::assertNotContains('device-0000', $ids);
        self::assertContains('device-new1', $ids);
    }

    public function testAnAiConnectionOnlyReads(): void
    {
        [$member, $id] = $this->session('ai@scutoons.test');
        $this->get(Roles::class)->grant($id, 'creator');
        $token = (string) self::json($this->request('POST', '/api/me/tokens', $member, ['name' => 'Claude']))['token'];
        $ai = ['Authorization' => 'Bearer ' . $token];
        $this->put($member, self::STUDIO, self::STATE);
        self::assertSame(self::STATE, self::json($this->request('GET', '/api/me/session?device=' . self::STUDIO, $ai))['mine']['state']);
        self::assertSame('403 not_for_ai', self::error($this->put($ai, self::STUDIO, self::STATE)));
    }

    public function testWithoutTheMigrationNothingThrows(): void
    {
        [$member] = $this->session('early@scutoons.test');
        $this->db->exec('DROP TABLE member_sessions');
        $state = self::json($this->request('GET', '/api/me/session?device=' . self::STUDIO, $member));
        self::assertSame([false, null, null], [$state['available'], $state['mine'], $state['other']]);
        self::assertSame('503 not_available', self::error($this->put($member, self::STUDIO, self::STATE)));
    }
}
