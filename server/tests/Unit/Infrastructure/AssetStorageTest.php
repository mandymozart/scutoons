<?php
declare(strict_types=1);

namespace Scutoons\Tests\Infrastructure;

use InvalidArgumentException;
use RuntimeException;
use Scutoons\Infrastructure\AssetStorage\BunnyConfig;
use Scutoons\Infrastructure\AssetStorage\BunnyStorage;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\AssetStorage\LocalStorage;
use Scutoons\Infrastructure\AssetStorage\StorageKey;
use Scutoons\Tests\TestCase;

/** The storage drivers of the asset layer: local folder, bunny.net against FakeBunny (the documented wire format) */
final class AssetStorageTest extends TestCase
{
    private function file(string $content): string
    {
        $path = $this->dir . '/src-' . bin2hex(random_bytes(4));
        file_put_contents($path, $content);
        return $path;
    }

    public function testStorageKeys(): void
    {
        self::assertSame('catpope/videos/a.mp4', StorageKey::normalize('/catpope//videos\\a.mp4'));
        self::assertSame('', StorageKey::normalize('', true));
        foreach (['../x.png', 'a/../../b.png', "a/\0b", ''] as $bad) {
            self::assertFalse(StorageKey::valid($bad), $bad);
        }
        $this->expectException(InvalidArgumentException::class);
        StorageKey::normalize('a/./b');
    }

    public function testLocalStorageRoundTrip(): void
    {
        $storage = new LocalStorage($this->dir . '/cdn', 'http://localhost:8001/');
        $stored = $storage->upload('catpope/images/a.webp', $this->file('AAAA'));
        self::assertSame(['catpope/images/a.webp', 4, 'image/webp'], [$stored->key, $stored->size, $stored->mime]);
        $storage->upload('catpope/.hidden/x.png', $this->file('x'));
        file_put_contents($this->dir . '/cdn/.htaccess', 'x');
        $storage->copy('catpope/images/a.webp', 'catpope/images/b.webp');
        $keys = array_map(fn($f) => $f->key, iterator_to_array($storage->list(''), false));
        sort($keys);
        self::assertSame(['catpope/images/a.webp', 'catpope/images/b.webp'], $keys, 'dot files / folders are not listed');
        self::assertSame(['catpope/images/a.webp', 'catpope/images/b.webp'], array_values(array_filter($keys, fn($k) => str_starts_with($k, 'catpope/images/'))));
        self::assertCount(0, iterator_to_array($storage->list('nope/'), false));
        self::assertSame('http://localhost:8001/catpope/images/a.webp', $storage->publicUrl('catpope/images/a.webp'));
        $storage->download('catpope/images/b.webp', $this->dir . '/back');
        self::assertSame('AAAA', file_get_contents($this->dir . '/back'));
        $storage->delete('catpope/images/a.webp');
        self::assertNull($storage->stat('catpope/images/a.webp'));
        self::assertTrue($storage->purge('catpope/images/b.webp'));
    }

    public function testBunnyStorageSpeaksTheDocumentedApi(): void
    {
        $fake = new FakeBunny('scutoons');
        $bunny = new BunnyStorage($fake, new BunnyConfig('scutoons', 'storage.bunnycdn.com', 'zone-password', 'read-only-password', '42', 'api-key', 'https://cdn.example'));
        $stored = $bunny->upload('catpope/videos/a b.720p.mp4', $this->file('video'));
        self::assertSame('PUT', $fake->requests[0]['method']);
        self::assertSame('https://storage.bunnycdn.com/scutoons/catpope/videos/a%20b.720p.mp4', $fake->requests[0]['url']);
        self::assertSame('zone-password', $fake->requests[0]['headers']['AccessKey'], 'writes: the read/write zone password, not the API key'); // test value
        self::assertSame(strtoupper(hash('sha256', 'video')), $fake->requests[0]['headers']['Checksum']);
        self::assertSame(hash('sha256', 'video'), $stored->sha256);

        $bunny->upload('catpope/images/x.webp', $this->file('img'));
        $bunny->upload('top.png', $this->file('t'));
        $files = [];
        foreach ($bunny->list('') as $file) $files[$file->key] = $file;
        ksort($files);
        self::assertSame(['catpope/images/x.webp', 'catpope/videos/a b.720p.mp4', 'top.png'], array_keys($files), 'recursive over directories');
        self::assertSame([3, hash('sha256', 'img'), 'image/webp'], [$files['catpope/images/x.webp']->size, $files['catpope/images/x.webp']->sha256, $files['catpope/images/x.webp']->mime]);
        self::assertSame('2026-10-02T10:00:00+00:00', $files['top.png']->modified);
        self::assertSame(['catpope/images/x.webp'], array_map(fn($f) => $f->key, iterator_to_array($bunny->list('catpope/images/'), false)));
        self::assertNotNull($bunny->stat('catpope/images/x.webp'));
        self::assertNull($bunny->stat('catpope/images/nope.webp'));
        $reads = array_filter($fake->requests, fn($r) => $r['method'] === 'GET');
        self::assertNotEmpty($reads);
        foreach ($reads as $r) self::assertSame('read-only-password', $r['headers']['AccessKey'], 'every read uses the read-only password'); // test value
        self::assertSame(['url' => 'https://cdn.example/top.png', 'status' => 200, 'size' => 1, 'cache' => 'HIT'], $bunny->headPublic('top.png'));

        $bunny->download('catpope/images/x.webp', $this->dir . '/x');
        self::assertSame('img', file_get_contents($this->dir . '/x'));
        $bunny->copy('catpope/images/x.webp', 'catpope/images/y.webp');
        self::assertSame('img', $fake->files['catpope/images/y.webp']);
        $bunny->delete('catpope/images/x.webp');
        self::assertArrayNotHasKey('catpope/images/x.webp', $fake->files);

        self::assertSame('https://cdn.example/catpope/videos/a%20b.720p.mp4', $bunny->publicUrl('catpope/videos/a b.720p.mp4'));
        self::assertTrue($bunny->purge('catpope/videos/a b.720p.mp4'));
        self::assertSame(['https://cdn.example/catpope/videos/a%20b.720p.mp4'], $fake->purged);
        self::assertSame(123456789, $bunny->statistics('2026-09-01', '2026-10-01')['TotalBandwidthUsed'] ?? null);
        self::assertStringContainsString('a.720p.mp4', (string) $bunny->requestLog('2026-10-01'));
        self::assertSame([42, 2, 'scutoons.b-cdn.net'], [($bunny->pullZone() ?? [])['Id'] ?? null, ($bunny->pullZone() ?? [])['OriginType'] ?? null, ($bunny->pullZone() ?? [])['Hostnames'][0]['Value'] ?? null]);
    }

    public function testBunnyWithoutApiKeyAndWriteGuard(): void
    {
        $fake = new FakeBunny('scutoons');
        $bunny = new BunnyStorage($fake, new BunnyConfig('scutoons', 'uk.storage.bunnycdn.com', 'zone-password', '', '', '', 'https://cdn.example', '_test/'));
        self::assertFalse($bunny->purge('x.png'), 'no API key: purge is off, not an error');
        self::assertNull($bunny->statistics('2026-09-01', '2026-10-01'));
        self::assertNull($bunny->requestLog('2026-10-01'));
        self::assertNull($bunny->pullZone());
        $bunny->upload('_test/ok.txt', $this->file('ok'));
        self::assertStringStartsWith('https://uk.storage.bunnycdn.com/scutoons/_test/', $fake->requests[0]['url']);
        try {
            $bunny->upload('catpope/images/x.webp', $this->file('no'));
            self::fail('a write outside BUNNY_WRITE_PREFIX');
        } catch (RuntimeException $error) {
            self::assertStringContainsString('_test/', $error->getMessage());
        }
        try {
            $bunny->delete('catpope/images/x.webp');
            self::fail('a delete outside BUNNY_WRITE_PREFIX');
        } catch (RuntimeException) {
        }
        self::assertSame(['_test/ok.txt'], array_keys($fake->files));
    }

    public function testBunnyErrorsNeverContainTheKeys(): void
    {
        $fake = new FakeBunny('scutoons', 'right-password');
        $bunny = new BunnyStorage($fake, new BunnyConfig('scutoons', 'storage.bunnycdn.com', '', 'wrong-password', '', '', ''));
        try {
            iterator_to_array($bunny->list(''));
            self::fail('401 expected');
        } catch (RuntimeException $error) {
            self::assertStringContainsString('HTTP 401', $error->getMessage());
            self::assertStringContainsString('wrong password', $error->getMessage());
            self::assertStringNotContainsString('wrong-password', $error->getMessage());
        }
        $other = new BunnyStorage(new FakeBunny('other-zone', 'zone-password'), new BunnyConfig('scutoons', 'storage.bunnycdn.com', 'zone-password', '', '', '', ''));
        try {
            $other->listDirectory('');
            self::fail('404 expected for the zone root');
        } catch (RuntimeException $error) {
            self::assertStringContainsString('wrong zone name or wrong region endpoint', $error->getMessage());
        }
        self::assertSame([], $other->listDirectory('missing-folder'), 'a missing sub-folder is just empty');
        try {
            $bunny->upload('x.png', $this->file('x'));
            self::fail('read-only without the read/write password');
        } catch (RuntimeException $error) {
            self::assertStringContainsString('read-only', $error->getMessage());
        }
    }

    public function testBunnyConfigIsTolerantAndExplains(): void
    {
        $config = BunnyConfig::fromSettings(Settings::fixed(self::root(), [
            'BUNNY_STORAGE_ZONE' => 'scutoons', 'BUNNY_STORAGE_ENDPOINT' => 'https://storage.bunnycdn.com/', 'BUNNY_STORAGE_READONLY_PASSWORD' => 'r',
            'BUNNY_PULLZONE_ID' => 'scutoons', 'CDN_PUBLIC_URL' => 'scutoons.b-cdn.net/',
        ]));
        self::assertSame(['storage.bunnycdn.com', 'https://scutoons.b-cdn.net', ''], [$config->endpoint, $config->publicBase, $config->pullZoneId]);
        self::assertSame('invalid', $config->status['BUNNY_PULLZONE_ID']['status'], 'a name instead of the numeric id');
        self::assertSame('missing', $config->status['BUNNY_STORAGE_PASSWORD']['status']);
        self::assertNull($config->status['BUNNY_STORAGE_READONLY_PASSWORD']['value'], 'passwords are never echoed');
        self::assertTrue($config->usable());
        self::assertSame('storage.bunnycdn.com', BunnyConfig::fromSettings(Settings::fixed(self::root(), []))->endpoint, 'the documented default');
        self::assertFalse(BunnyConfig::fromSettings(Settings::fixed(self::root(), ['BUNNY_STORAGE_ZONE' => 'z']))->usable());
        self::assertSame('https://cdn.scutoons.com', BunnyConfig::baseUrl('https://cdn.scutoons.com/'));
        self::assertSame('', BunnyConfig::baseUrl('not a url'));
        self::assertSame('6731366', BunnyConfig::fromSettings(Settings::fixed(self::root(), ['BUNNY_PULLZONE_ID' => ' 6731366 ']))->pullZoneId);
    }
}
