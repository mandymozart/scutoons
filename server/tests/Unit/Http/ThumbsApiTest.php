<?php
declare(strict_types=1);

namespace Scutoons\Tests\Http;

use Psr\Http\Message\ResponseInterface;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Domain\Thumbnails\Thumbnails;
use Scutoons\Http\EditorTokenScope;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Kernel;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

/**
 * Thumbnails (Domain\Thumbnails, #842): one 264 × 168 WebP per key – shared addresses and assets (content editors upload),
 * a member's own workspace snapshots (nobody else sees them); an asset's still is its `thumb` rendition, made stale by a
 * new version, which queues the server's job for images.
 */
final class ThumbsApiTest extends TestCase
{
    protected function settings(): array
    {
        return ['ASSET_STORAGE' => 'local', 'ASSET_LOCAL_ROOT' => $this->dir . '/cdn', 'CDN_URL' => 'http://cdn.test/'] + parent::settings();
    }

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

    /** @return array{0: array<string, string>, 1: string} */
    private function as(string $email, ?string $role = null): array
    {
        $user = $this->user($email);
        if ($role !== null) $this->get(Roles::class)->grant((string) $user['id'], $role);
        return [['Authorization' => 'Bearer ' . $this->get(Sessions::class)->create((string) $user['id'])], (string) $user['id']];
    }

    /** A WebP header (VP8X) of the given canvas – enough for the size check; the bytes are stored as they come */
    private static function webp(int $w = 264, int $h = 168, string $tag = ''): string
    {
        $chunk = 'VP8X' . pack('V', 10) . "\x00\x00\x00\x00" . substr(pack('V', $w - 1), 0, 3) . substr(pack('V', $h - 1), 0, 3) . $tag;
        return 'RIFF' . pack('V', 4 + strlen($chunk)) . 'WEBP' . $chunk;
    }

    private function file(string $url): string
    {
        $path = (string) parse_url($url, PHP_URL_PATH);
        return $this->dir . '/cdn' . $path;
    }

    public function testTheWebpCheck(): void
    {
        self::assertSame([264, 168], Thumbnails::webpSize(self::webp()));
        self::assertNull(Thumbnails::webpSize("\x89PNG\r\n\x1a\n" . str_repeat("\0", 30)));
        // a real lossy VP8 header (cwebp, 264 × 168)
        $vp8 = 'RIFF' . pack('V', 100) . 'WEBPVP8 ' . pack('V', 80) . "\x10\x02\x00\x9d\x01\x2a" . pack('v', 264) . pack('v', 168);
        self::assertSame([264, 168], Thumbnails::webpSize($vp8));
        foreach (['asset:nope', 'workspace:Bad', 'nothing', 'item:../x', 'item:'] as $bad) {
            try {
                Thumbnails::kindOf($bad);
                self::fail($bad);
            } catch (ApiError $e) {
                self::assertSame(400, $e->status);
            }
        }
        self::assertSame(['address', 'address', 'workspace'], [Thumbnails::kindOf('item:camera'), Thumbnails::kindOf('character:bob/spot:hand'), Thumbnails::kindOf('workspace:character')]);
    }

    public function testSharedStillsAreUploadedByEditorsAndReplaced(): void
    {
        [$member] = $this->as('mira@scutoons.test');
        [$creator] = $this->as('ced@scutoons.test', 'creator');
        self::assertSame(401, $this->request('GET', '/api/thumbs?keys=item:camera')->getStatusCode());
        self::assertSame(['available' => true, 'thumbs' => []], array_intersect_key(self::json($this->request('GET', '/api/thumbs?keys=item:camera', $member)), ['available' => 1, 'thumbs' => 1]));
        self::assertSame(400, $this->request('GET', '/api/thumbs?keys=bad', $member)->getStatusCode());

        self::assertSame(403, $this->request('PUT', '/api/thumbs?key=item:camera', $member, self::webp())->getStatusCode());
        self::assertSame('bad_thumb', self::json($this->request('PUT', '/api/thumbs?key=item:camera', $creator, self::webp(300, 200)))['error']);
        self::assertSame('bad_thumb', self::json($this->request('PUT', '/api/thumbs?key=item:camera', $creator, str_repeat('x', 64)))['error']);
        self::assertSame(413, $this->request('PUT', '/api/thumbs?key=item:camera', $creator, self::webp() . str_repeat("\0", Thumbnails::MAX_BYTES))->getStatusCode());

        $r = $this->request('PUT', '/api/thumbs?key=item:camera&stamp=2026-11-24T10:00:00Z', $creator, self::webp(tag: 'one'));
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
        $first = self::json($r)['thumb'];
        self::assertSame(['item:camera', '2026-11-24T10:00:00Z', false, 'client'], [$first['key'], $first['stamp'], $first['stale'], $first['origin']]);
        self::assertStringContainsString('/.thumbs/shared/', $first['url']);
        self::assertFileExists($this->file($first['url']));

        // every member reads it; a new upload gets a new name and the old file goes
        $seen = self::json($this->request('GET', '/api/thumbs?keys=item:camera,scene:nope', $member))['thumbs'];
        self::assertSame(['item:camera'], array_keys($seen));
        $second = self::json($this->request('PUT', '/api/thumbs?key=item:camera&stamp=2', $creator, self::webp(tag: 'two')))['thumb'];
        self::assertNotSame($first['url'], $second['url']);
        self::assertFileDoesNotExist($this->file($first['url']));
        self::assertStringEndsWith('two', (string) file_get_contents($this->file($second['url'])));
    }

    public function testWorkspaceSnapshotsArePrivate(): void
    {
        [$mira] = $this->as('mira@scutoons.test');
        [$otto] = $this->as('otto@scutoons.test');
        $r = $this->request('PUT', '/api/thumbs?key=workspace:character', $mira, self::webp(tag: 'mira'));
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
        $mine = self::json($r)['thumb'];
        self::assertStringContainsString('/.thumbs/members/', $mine['url']);
        self::assertSame([], self::json($this->request('GET', '/api/thumbs?keys=workspace:character', $otto))['thumbs']);
        $ottos = self::json($this->request('PUT', '/api/thumbs?key=workspace:character', $otto, self::webp(tag: 'otto')))['thumb'];
        self::assertSame($mine['url'], self::json($this->request('GET', '/api/thumbs?keys=workspace:character', $mira))['thumbs']['workspace:character']['url']);
        self::assertSame($ottos['url'], self::json($this->request('GET', '/api/thumbs?keys=workspace:character', $otto))['thumbs']['workspace:character']['url']);
    }

    public function testAnAssetsStillIsItsThumbRenditionAndGoesStaleWithANewVersion(): void
    {
        [$creator] = $this->as('ced@scutoons.test', 'creator');
        $png = "\x89PNG\r\n\x1a\n\0\0\0\rIHDR\0\0\0\x01\0\0\0\x01\x08\x02\0\0\0\0\0\0\0";
        $up = $this->request('PUT', '/api/assets/upload?path=catpope/images/pic.png', $creator + ['Content-Type' => 'image/png'], $png);
        self::assertSame(201, $up->getStatusCode(), (string) $up->getBody());
        $id = (string) self::json($up)['asset']['id'];
        $db = $this->get(Database::class);
        // the server draws an image's still: a job is queued, once
        self::assertSame(1, (int) $db->value("SELECT COUNT(*) FROM asset_jobs WHERE asset_id = ? AND kind = 'thumb' AND status = 'queued'", [$id]));

        $put = $this->request('PUT', "/api/thumbs?key=asset:$id", $creator, self::webp(tag: 'v1'));
        self::assertSame(200, $put->getStatusCode(), (string) $put->getBody());
        $thumb = self::json($put)['thumb'];
        self::assertSame(['1', false], [$thumb['stamp'], $thumb['stale']]);
        self::assertStringContainsString('/catpope/images/pic.thumb.webp?v=', $thumb['url']);
        $labels = array_column(self::json($this->request('GET', "/api/assets/$id", $creator))['versions'][0]['renditions'], 'label');
        self::assertContains('thumb', $labels);
        // the list (the Library) does not show the still as a file of its own
        $listed = self::json($this->request('GET', '/api/assets?q=pic', $creator))['items'][0];
        self::assertNotContains('thumb', array_column($listed['renditions'], 'label'));

        // replacing the source: stale; still one queued job
        $r = $this->request('PUT', '/api/assets/replace?path=catpope/images/pic.png', $creator + ['Content-Type' => 'image/png'], $png . 'v2');
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
        $after = self::json($this->request('GET', "/api/thumbs?keys=asset:$id", $creator))['thumbs']["asset:$id"];
        self::assertTrue($after['stale']);
        self::assertSame(1, (int) $db->value("SELECT COUNT(*) FROM asset_jobs WHERE asset_id = ? AND kind = 'thumb' AND status = 'queued'", [$id]));
        // a new still for the new version
        $again = self::json($this->request('PUT', "/api/thumbs?key=asset:$id", $creator, self::webp(tag: 'v2')))['thumb'];
        self::assertSame(['2', false], [$again['stamp'], $again['stale']]);
        self::assertNotSame($thumb['url'], $again['url']);

        // a model gets no job: the browser renders it
        $glb = $this->request('PUT', '/api/assets/upload?path=scutoons/3d/lantern.glb', $creator + ['Content-Type' => 'model/gltf-binary'], "glTF\x02\0\0\0\x0c\0\0\0");
        $model = (string) self::json($glb)['asset']['id'];
        self::assertSame(0, (int) $db->value("SELECT COUNT(*) FROM asset_jobs WHERE asset_id = ?", [$model]));

        // the stills of what was there before: queued for images without a fresh one, once
        $other = $this->request('PUT', '/api/assets/upload?path=catpope/images/old.png', $creator + ['Content-Type' => 'image/png'], $png);
        $old = (string) self::json($other)['asset']['id'];
        $db->run("DELETE FROM asset_jobs WHERE kind = 'thumb'");
        self::assertSame(1, $this->get(Thumbnails::class)->queueMissing());
        self::assertSame(1, (int) $db->value("SELECT COUNT(*) FROM asset_jobs WHERE asset_id = ? AND kind = 'thumb'", [$old]));
        self::assertSame(0, $this->get(Thumbnails::class)->queueMissing());
    }

    public function testTheEditorTokenMayUploadStills(): void
    {
        EditorTokenScope::guard('PUT', '/api/thumbs');
        EditorTokenScope::guard('GET', '/api/thumbs');
        $this->expectException(ApiError::class);
        EditorTokenScope::guard('POST', '/api/thumbs');
    }
}
