<?php
declare(strict_types=1);

namespace Scutoons\Tests\Http;

use Psr\Http\Message\ResponseInterface;
use Scutoons\Domain\Assets\AssetImporter;
use Scutoons\Domain\Assets\AssetUsages;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Domain\Content\EntityKey;
use Scutoons\Domain\Content\EntityStore;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Infrastructure\AssetStorage\AssetStorages;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Kernel;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

/** /api/assets/* (content editors) and /api/admin/assets/* (admins) */
final class AssetsApiTest extends TestCase
{
    protected function settings(): array
    {
        return [
            'ASSET_STORAGE' => 'local',
            'ASSET_LOCAL_ROOT' => $this->dir . '/cdn',
            'CDN_URL' => 'http://cdn.test/',
            'BUNNY_STORAGE_ZONE' => 'zone',
            'BUNNY_STORAGE_READONLY_PASSWORD' => 'never-shown-password',
            'BUNNY_PROTECTED_STORAGE_ZONE' => 'vault-zone',
            'BUNNY_PROTECTED_CDN_KEY' => 'never-shown-vault-key',
        ] + parent::settings();
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

    /** @return array<string, string> */
    private function as(string $role): array
    {
        $user = $this->user($role . '@example.com');
        if ($role !== 'member') $this->get(Roles::class)->grant((string) $user['id'], $role);
        return ['Authorization' => 'Bearer ' . $this->get(Sessions::class)->create((string) $user['id'])];
    }

    private function fixture(): void
    {
        @mkdir($this->dir . '/cdn/catpope/videos', 0777, true);
        foreach (['480p', '720p'] as $m) file_put_contents($this->dir . "/cdn/catpope/videos/clip.$m.mp4", "\0\0\0\x18ftypmp42$m");
        $this->get(EntityStore::class)->insertPublished(EntityKey::parse('chapter:catpope:1:de'), '{"groups":[{"slices":[{"video":{"src":"videos/clip.720p.mp4"}}]}]}', null);
        $this->get(AssetImporter::class)->import($this->get(AssetStorages::class)->local());
        $this->get(AssetUsages::class)->rebuild();
    }

    public function testRoles(): void
    {
        $member = $this->as('member');
        $creator = $this->as('creator');
        self::assertSame(401, $this->request('GET', '/api/assets')->getStatusCode());
        self::assertSame(403, $this->request('GET', '/api/assets', $member)->getStatusCode());
        self::assertSame(200, $this->request('GET', '/api/assets', $creator)->getStatusCode());
        foreach ([['GET', '/api/admin/assets/storage'], ['POST', '/api/admin/assets/scan'], ['GET', '/api/admin/assets/jobs'], ['GET', '/api/admin/assets/unused'], ['POST', '/api/admin/assets/cleanup']] as [$method, $path]) {
            self::assertSame(403, $this->request($method, $path, $creator)->getStatusCode(), $path);
        }
    }

    public function testBrowseLookupUploadAndTheDeleteCheck(): void
    {
        $this->fixture();
        $creator = $this->as('creator');
        $list = self::json($this->request('GET', '/api/assets?kind=video', $creator));
        self::assertSame(1, $list['pagination']['total']);
        $item = $list['items'][0];
        self::assertSame(['catpope/videos/clip', 'video', 1], [$item['slug'], $item['kind'], $item['usages']]);
        self::assertSame(['480p', '720p'], array_column($item['renditions'], 'label'));
        self::assertSame('http://cdn.test/', $list['cdn']);
        self::assertSame(400, $this->request('GET', '/api/assets?kind=nope', $creator)->getStatusCode());
        self::assertSame(0, self::json($this->request('GET', '/api/assets?state=unused', $creator))['pagination']['total']);
        // resolution: an asset with a file at that level
        self::assertSame(1, self::json($this->request('GET', '/api/assets?res=480p', $creator))['pagination']['total']);
        self::assertSame(0, self::json($this->request('GET', '/api/assets?res=1080p', $creator))['pagination']['total']);
        self::assertSame(400, $this->request('GET', '/api/assets?res=nope', $creator)->getStatusCode());
        // a density level (no pixel band) by its label only: the clip has none
        self::assertSame(0, self::json($this->request('GET', '/api/assets?res=2x', $creator))['pagination']['total']);
        // the clip has two files: not "the original only"
        self::assertSame(0, self::json($this->request('GET', '/api/assets?files=single', $creator))['pagination']['total']);
        self::assertSame(1, self::json($this->request('GET', '/api/assets?files=several', $creator))['pagination']['total']);
        self::assertSame(400, $this->request('GET', '/api/assets?files=nope', $creator)->getStatusCode());

        $lookup = self::json($this->request('GET', '/api/assets/lookup?path=catpope/videos/clip.480p.mp4', $creator));
        self::assertSame(['low', 'catpope/videos/clip'], [$lookup['rendition']['quality'], $lookup['asset']['asset']['slug']]);
        self::assertNull(self::json($this->request('GET', '/api/assets/lookup?path=catpope/nope.png', $creator))['asset']);

        $png = "\x89PNG\r\n\x1a\n\0\0\0\rIHDR\0\0\0\x01\0\0\0\x01\x08\x02\0\0\0\0\0\0\0";
        $up = $this->request('PUT', '/api/assets/upload?path=catpope/images/new.png&title=New&tags=cover,art', $creator + ['Content-Type' => 'image/png'], $png);
        self::assertSame(201, $up->getStatusCode(), (string) $up->getBody());
        self::assertSame(['catpope/images/new', ['cover', 'art']], [self::json($up)['asset']['slug'], self::json($up)['asset']['tags']]);
        self::assertFileExists($this->dir . '/cdn/catpope/images/new.png');
        self::assertSame(409, $this->request('PUT', '/api/assets/upload?path=catpope/images/new.png', $creator + ['Content-Type' => 'image/png'], $png)->getStatusCode());
        self::assertSame(422, $this->request('PUT', '/api/assets/upload?path=catpope/images/empty.png', $creator + ['Content-Type' => 'image/png'], '')->getStatusCode());

        // a creator replaces a file in place: a new version, the old bytes archived; same bytes / unknown path refused
        $png2 = $png . 'v2';
        $replaced = $this->request('PUT', '/api/assets/replace?path=catpope/images/new.png', $creator + ['Content-Type' => 'image/png'], $png2);
        self::assertSame(200, $replaced->getStatusCode(), (string) $replaced->getBody());
        self::assertSame('catpope/images/new.png', self::json($replaced)['replaced']);
        self::assertSame($png2, file_get_contents($this->dir . '/cdn/catpope/images/new.png'));
        self::assertFileExists($this->dir . '/cdn/' . self::json($replaced)['archived_as']);
        self::assertSame(409, $this->request('PUT', '/api/assets/replace?path=catpope/images/new.png', $creator + ['Content-Type' => 'image/png'], $png2)->getStatusCode());
        self::assertSame(404, $this->request('PUT', '/api/assets/replace?path=catpope/images/nope.png', $creator + ['Content-Type' => 'image/png'], $png2)->getStatusCode());
        self::assertSame(403, $this->request('PUT', '/api/assets/replace?path=catpope/images/new.png', $this->as('member') + ['Content-Type' => 'image/png'], $png2)->getStatusCode());

        $id = (string) $item['id'];
        $meta = self::json($this->request('PUT', "/api/assets/$id", $creator + ['Content-Type' => 'application/json'], '{"title":"The clip","tags":["prologue"]}'));
        self::assertSame(['The clip', ['prologue']], [$meta['asset']['title'], $meta['asset']['tags']]);

        // the vector library (agents/research/VECTORS.md "The API"): an SVG + its shape row; kind=shape lists only those
        $svg = '<svg width="296" height="1422" viewBox="0 0 296 1422" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M0 0h1v1z" fill="#FF0000"/></svg>';
        $shapeUp = $this->request('PUT', '/api/assets/upload?path=catpope/vectors/crescent-chain.svg&tags=band,crescent', $creator + ['Content-Type' => 'image/svg+xml'], $svg);
        self::assertSame(201, $shapeUp->getStatusCode(), (string) $shapeUp->getBody());
        $shapeId = (string) self::json($shapeUp)['asset']['id'];
        self::assertSame(0, self::json($this->request('GET', '/api/assets?kind=shape', $creator))['pagination']['total'], 'an SVG alone is no shape');
        $shape = ['tile' => 'y', 'mask' => true, 'colors' => ['#ff0000'], 'source' => ['file' => 'Cat Pope', 'node' => '2819:3886']];
        $set = $this->request('PUT', "/api/assets/$shapeId", $creator + ['Content-Type' => 'application/json'], (string) json_encode(['shape' => $shape]));
        self::assertSame(200, $set->getStatusCode(), (string) $set->getBody());
        self::assertSame(['tile' => 'y', 'mask' => true, 'colors' => ['#FF0000'], 'source' => ['file' => 'Cat Pope', 'node' => '2819:3886'], 'derived' => null], self::json($set)['asset']['shape']);
        $shapes = self::json($this->request('GET', '/api/assets?kind=shape&folder=catpope', $creator));
        self::assertSame(1, $shapes['pagination']['total']);
        $row = $shapes['items'][0];
        self::assertSame(['catpope/vectors/crescent-chain.svg', ['band', 'crescent'], 296, 1422, ['tile' => 'y', 'mask' => true, 'colors' => ['#FF0000']]],
            [$row['path'], $row['tags'], $row['width'], $row['height'], $row['shape']]);
        self::assertArrayNotHasKey('shape', self::json($this->request('GET', '/api/assets?kind=video', $creator))['items'][0]);
        self::assertSame(422, $this->request('PUT', "/api/assets/$shapeId", $creator + ['Content-Type' => 'application/json'], '{"shape":{"tile":"z"}}')->getStatusCode());
        self::assertSame(422, $this->request('PUT', "/api/assets/$shapeId", $creator + ['Content-Type' => 'application/json'], '{"shape":{"colors":["red"]}}')->getStatusCode());
        $unset = self::json($this->request('PUT', "/api/assets/$shapeId", $creator + ['Content-Type' => 'application/json'], '{"shape":null}'));
        self::assertArrayNotHasKey('shape', $unset['asset']);
        self::assertSame(0, self::json($this->request('GET', '/api/assets?kind=shape', $creator))['pagination']['total']);

        // a creator save re-indexes the entity: the chapter now uses the 480p file instead
        $put = $this->request('PUT', '/api/entities/chapter:catpope:1:de', $creator + ['Content-Type' => 'application/json'], '{"content":' . self::chapterJson(['groups' => [['slices' => [['image' => 'images/s.webp', 'width' => 390, 'height' => 600, 'video' => ['src' => 'videos/clip.480p.mp4']]]]]]) . '}');
        self::assertSame(200, $put->getStatusCode(), (string) $put->getBody());
        $detail = self::json($this->request('GET', "/api/assets/$id", $creator));
        self::assertEqualsCanonicalizing([['catpope/videos/clip.720p.mp4', 'published'], ['catpope/videos/clip.480p.mp4', 'draft']], array_map(fn($u) => [$u['path'], $u['state']], $detail['usages']));

        $admin = $this->as('admin');
        $check = self::json($this->request('GET', "/api/admin/assets/$id/delete-check", $admin));
        self::assertSame([false, 1, 1], [$check['can_delete'], $check['blocking'], $check['drafts']]);
        $refused = $this->request('DELETE', "/api/admin/assets/$id?force=1", $admin);
        self::assertSame([409, 'in_use'], [$refused->getStatusCode(), self::json($refused)['error']]);
        self::assertFileExists($this->dir . '/cdn/catpope/videos/clip.720p.mp4');
    }

    /** a score's file goes into the protected storage (agents/research/SCORES.md §5): editors only, checked type, a hashed name */
    public function testScoreFilesGoIntoTheProtectedStorage(): void
    {
        $member = $this->as('member');
        $creator = $this->as('creator');
        $xml = '<?xml version="1.0"?><score-partwise version="4.0"><part-list/></score-partwise>';
        self::assertSame(403, $this->request('PUT', '/api/assets/protected?item=cloud9_en&file=musicxml', $member, $xml)->getStatusCode());
        self::assertSame('wrong_type', self::json($this->request('PUT', '/api/assets/protected?item=cloud9_en&file=pdf', $creator, $xml))['error'] ?? null);
        self::assertSame('invalid_item', self::json($this->request('PUT', '/api/assets/protected?item=../x&file=pdf', $creator, '%PDF-1.7'))['error'] ?? null);
        $response = $this->request('PUT', '/api/assets/protected?item=cloud9_en&file=musicxml', $creator, $xml);
        self::assertSame(201, $response->getStatusCode());
        $path = (string) (self::json($response)['path'] ?? '');
        self::assertSame('scores/cloud9_en/' . substr(hash('sha256', $xml), 0, 12) . '.musicxml', $path);
        self::assertSame($xml, file_get_contents($this->dir . "/cdn/$path"), 'dev: the local media folder');
        self::assertSame(201, $this->request('PUT', '/api/assets/protected?item=cloud9_en&file=pdf', $creator, "%PDF-1.7\n")->getStatusCode());
        self::assertSame(0, (int) $this->db->value("SELECT COUNT(*) FROM asset_versions WHERE path LIKE 'scores/%'"), 'never in the public asset registry');
    }

    public function testStorageStatusNeverShowsSecrets(): void
    {
        $this->fixture();
        $admin = $this->as('admin');
        $response = $this->request('GET', '/api/admin/assets/storage', $admin);
        $body = self::json($response);
        self::assertSame('local', $body['primary']);
        self::assertSame(['assets' => 1, 'files' => 2], array_intersect_key($body['totals'], ['assets' => 0, 'files' => 0]));
        self::assertSame('set', $body['settings']['BUNNY_STORAGE_READONLY_PASSWORD']['status']);
        self::assertStringNotContainsString('never-shown-password', (string) $response->getBody());
        // the vault in the same list: its zone shown, its key only as "set"
        self::assertSame(['set', 'vault-zone'], [$body['settings']['BUNNY_PROTECTED_STORAGE_ZONE']['status'], $body['settings']['BUNNY_PROTECTED_STORAGE_ZONE']['value']]);
        self::assertSame(['missing', 'set'], [$body['settings']['BUNNY_PROTECTED_STORAGE_PASSWORD']['status'], $body['settings']['BUNNY_PROTECTED_CDN_KEY']['status']]);
        self::assertStringNotContainsString('never-shown-vault-key', (string) $response->getBody());
        self::assertFalse($body['bunny']['can_write']);

        $scan = self::json($this->request('POST', '/api/admin/assets/scan', $admin));
        self::assertSame(0, $scan['import']['assets_created']);
        self::assertSame(1, $scan['usages']['content']);
    }

    public function testUnusedAndCleanup(): void
    {
        $this->fixture();
        $admin = $this->as('admin');
        $json = $admin + ['Content-Type' => 'application/json'];
        $unused = self::json($this->request('GET', '/api/admin/assets/unused', $admin));
        self::assertSame(['catpope/videos/clip.480p.mp4'], array_column($unused['items'][0]['files'] ?? [], 'path'), 'the 720p is used');
        self::assertSame(1, $unused['totals']['files']);
        self::assertSame(422, $this->request('POST', '/api/admin/assets/cleanup', $json, '{"paths":[]}')->getStatusCode());
        $result = self::json($this->request('POST', '/api/admin/assets/cleanup', $json, (string) json_encode(['paths' => ['catpope/videos/clip.480p.mp4', 'catpope/videos/clip.720p.mp4']])));
        self::assertSame(['catpope/videos/clip.480p.mp4'], $result['deleted']);
        self::assertSame([['path' => 'catpope/videos/clip.720p.mp4', 'reason' => 'in_use_or_unknown']], $result['skipped']);
        self::assertFileDoesNotExist($this->dir . '/cdn/catpope/videos/clip.480p.mp4');
        self::assertFileExists($this->dir . '/cdn/catpope/videos/clip.720p.mp4');
    }
}
