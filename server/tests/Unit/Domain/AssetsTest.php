<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Assets\AssetImporter;
use Scutoons\Domain\Assets\AssetJobs;
use Scutoons\Domain\Assets\AssetLayout;
use Scutoons\Domain\Assets\AssetPaths;
use Scutoons\Domain\Assets\AssetRegistry;
use Scutoons\Domain\Assets\AssetService;
use Scutoons\Domain\Assets\AssetUsages;
use Scutoons\Domain\Assets\MediaProbe;
use Scutoons\Domain\Content\EntityKey;
use Scutoons\Domain\Content\EntityStore;
use Scutoons\Infrastructure\AssetStorage\AssetStorages;
use Scutoons\Tests\TestCase;

/** The media asset layer: grouping, import, usage index, upload / replace / relink / rename / delete check */
final class AssetsTest extends TestCase
{
    /** smallest valid files per type (the upload checks the magic bytes) */
    private const PNG = "\x89PNG\r\n\x1a\n\0\0\0\rIHDR\0\0\0\x02\0\0\0\x01\x08\x02\0\0\0\0\0\0\0";
    private const MP4 = "\0\0\0\x18ftypmp42\0\0\0\0mp42isom";
    private const WEBP = "RIFF\x1a\0\0\0WEBPVP8 \x0e\0\0\0";

    protected function settings(): array
    {
        $catalog = $this->dir . '/items.json';
        file_put_contents($catalog, json_encode(['items' => [['id' => 'vinyl', 'icon' => 'catpope/items/vinyl.png']], 'albums' => [['id' => 'a', 'download' => 'catpope/audio/album.zip']]], JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES));
        foreach (['products', 'editions', 'toons', 'featured'] as $name) file_put_contents("{$this->dir}/$name.json", '{}');
        return [
            'ASSET_STORAGE' => 'local',
            'ASSET_LOCAL_ROOT' => $this->dir . '/cdn',
            'CDN_URL' => 'http://cdn.test/',
            'ITEMS_PATH' => $catalog,
            'PRODUCTS_PATH' => $this->dir . '/products.json',
            'EDITIONS_PATH' => $this->dir . '/editions.json',
            'TOONS_PATH' => $this->dir . '/toons.json',
            'FEATURED_PRODUCTS_PATH' => $this->dir . '/featured.json',
        ] + parent::settings();
    }

    private function put(string $key, string $bytes): void
    {
        $file = $this->dir . '/cdn/' . $key;
        @mkdir(dirname($file), 0777, true);
        file_put_contents($file, $bytes);
    }

    private function local(string $bytes): string
    {
        $file = $this->dir . '/up-' . bin2hex(random_bytes(4));
        file_put_contents($file, $bytes);
        return $file;
    }

    /** @param array<mixed> $doc */
    private function entity(string $key, array $doc, bool $publish = true): int
    {
        $store = $this->get(EntityStore::class);
        $json = (string) json_encode($doc, JSON_UNESCAPED_SLASHES);
        return $publish ? $store->insertPublished(EntityKey::parse($key), $json, null) : $store->insert(EntityKey::parse($key), $json, null, 'draft', null);
    }

    /** a small CDN + content: the trailer + prologue videos, a slice, an item icon, the album zip */
    private function fixture(): void
    {
        foreach (['480p', '720p', '1080p', '1280w', '1920w'] as $m) $this->put("catpope/videos/s1-trailer-loop.$m.mp4", self::MP4 . $m);
        foreach (['480p', '720p', '1080p'] as $m) $this->put("catpope/videos/roar-little-kitty.$m.mp4", self::MP4 . 'rlk' . $m);
        $this->put('catpope/images/webtoon/s1c1/s1c1-slice-1.webp', self::WEBP);
        $this->put('catpope/images/s1-cover-square.jpg', "\xFF\xD8\xFFjpg");
        $this->put('catpope/images/s1-cover-square.webp', self::WEBP . 'c');
        $this->put('catpope/images/1x/s1c3-key-visual.jpg', "\xFF\xD8\xFF1x");
        $this->put('catpope/images/3x/s1c3-key-visual.jpg', "\xFF\xD8\xFF3x");
        $this->put('catpope/audio/music/s1c1.128.mp3', 'ID3a');
        $this->put('catpope/audio/music/s1c1.320.mp3', 'ID3b');
        $this->put('catpope/items/vinyl.png', self::PNG);
        $this->put('catpope/audio/album.zip', "PK\x03\x04zip");
        $this->put('characters/catpope/catpope_portrait.webp', self::WEBP . 'p');
        $this->put('catpope/.hidden/x.png', self::PNG);
        $this->put('catpope/Thumbs.db', 'junk');

        $this->entity('season:catpope:1:de', ['trailer' => ['loop' => 'videos/s1-trailer-loop.720p.mp4', 'variants' => ['low' => 'videos/s1-trailer-loop.480p.mp4', 'high' => 'videos/s1-trailer-loop.1080p.mp4'], 'poster' => 'images/missing-poster.webp'], 'cover' => ['square' => 'images/s1-cover-square.webp']]);
        $this->entity('chapter:catpope:1:de', ['groups' => [['slices' => [['image' => 'images/webtoon/s1c1/s1c1-slice-1.webp']]]], 'tile' => '../characters/catpope/catpope_portrait.webp']);
        $this->entity('chapter:catpope:1:de', ['groups' => [['slices' => [['image' => 'images/webtoon/s1c1/s1c1-slice-1.webp'], ['image' => 'images/s1-cover-square.jpg']]]]], false);
        $this->entity('map', ['locations' => [['unlock' => ['toon' => 'catpope', 'slice' => 'images/webtoon/s1c1/s1c1-slice-1.webp']]]]);
        $this->entity('asset-requests', ['requests' => [['path' => 'catpope/images/wanted.webp']]]);
    }

    private function scan(): void
    {
        $this->get(AssetImporter::class)->import($this->get(AssetStorages::class)->local());
        $this->get(AssetUsages::class)->rebuild();
    }

    private function assetId(string $slug): string
    {
        $asset = $this->get(AssetRegistry::class)->bySlug($slug);
        self::assertNotNull($asset, $slug);
        return (string) $asset['id'];
    }

    private function versionId(string $slug): int
    {
        $version = $this->get(AssetRegistry::class)->currentVersion($this->assetId($slug));
        self::assertNotNull($version, $slug);
        return (int) $version['id'];
    }

    public function testGrouping(): void
    {
        self::assertSame(['slug' => 'catpope/videos/x', 'marker' => '720p', 'quality' => 'medium', 'ext' => 'mp4', 'kind' => 'video', 'rank' => 720], AssetPaths::parse('catpope/videos/x.720p.mp4'));
        self::assertSame('catpope/images/k', AssetPaths::parse('catpope/images/3x/k.jpg')['slug']);
        self::assertSame(['3x', 'high'], [AssetPaths::parse('catpope/images/3x/k.jpg')['marker'], AssetPaths::parse('catpope/images/3x/k.jpg')['quality']]);
        self::assertSame(['128k', 'medium'], [AssetPaths::parse('a/m.128.mp3')['marker'], AssetPaths::parse('a/m.128.mp3')['quality']]);
        self::assertSame('a/x', AssetPaths::parse('a/x@2x.png')['slug']);
        self::assertSame('poster', AssetPaths::parse('a/v.poster.webp')['marker']);
        self::assertNull(AssetPaths::parse('a/v2.mp4')['marker'], 'no marker in a plain name');

        $groups = AssetPaths::group(['v/t.480p.mp4', 'v/t.1080p.mp4', 'v/t.1920w.mp4', 'i/c.jpg', 'i/c.webp', 'i/only.webp']);
        self::assertSame(['i/c', 'i/only', 'v/t'], array_keys($groups));
        self::assertSame('v/t.1920w.mp4', $groups['v/t']['source'], 'the biggest marker is the source');
        self::assertSame(['label' => '480p', 'quality' => 'low'], $groups['v/t']['files']['v/t.480p.mp4']);
        self::assertSame('i/c.jpg', $groups['i/c']['source'], 'the delivered format, not its WebP twin');
        self::assertSame('webp', $groups['i/c']['files']['i/c.webp']['label']);
        self::assertSame('original', $groups['i/only']['files']['i/only.webp']['label']);

        self::assertSame('characters/x.webp', AssetPaths::resolve('../characters/x.webp', 'catpope'));
        self::assertSame('catpope/images/a.webp', AssetPaths::resolve('images/a.webp', 'catpope'));
        self::assertSame('catpope/images/a.webp', AssetPaths::resolve('https://cdn.scutoons.com/catpope/images/a.webp', '', ['https://cdn.scutoons.com']));
        self::assertNull(AssetPaths::resolve('https://elsewhere.example/a.webp', ''));
        self::assertNull(AssetPaths::resolve('../../x.png', 'catpope'));
        self::assertSame('videos/b.mp4', AssetPaths::relativeTo('catpope/videos/b.mp4', 'catpope', 'videos/a.mp4'));
        self::assertSame('../characters/b.webp', AssetPaths::relativeTo('characters/b.webp', 'catpope', 'images/a.webp'));
        self::assertSame('catpope/items/b.png', AssetPaths::relativeTo('catpope/items/b.png', '', 'catpope/items/a.png'));
        self::assertSame('catpope/art/1x/b.jpg', AssetService::renamedPath('catpope/images/1x/a.jpg', 'catpope/images/a', 'catpope/art/b'));
        self::assertSame('catpope/videos/new.720p.mp4', AssetService::renamedPath('catpope/videos/old.720p.mp4', 'catpope/videos/old', 'catpope/videos/new'));
    }

    public function testImportIsIdempotentAndGroupsRenditions(): void
    {
        $this->fixture();
        $importer = $this->get(AssetImporter::class);
        $stats = $importer->import($this->get(AssetStorages::class)->local());
        self::assertSame(18, $stats['files'], 'dot folders and Thumbs.db are skipped');
        self::assertSame(9, $stats['assets_created']);
        $again = $importer->import($this->get(AssetStorages::class)->local());
        self::assertSame([0, 0, 0, 0], [$again['assets_created'], $again['renditions_added'], $again['renditions_updated'], $again['renditions_removed']]);

        $registry = $this->get(AssetRegistry::class);
        $trailer = $registry->bySlug('catpope/videos/s1-trailer-loop');
        self::assertNotNull($trailer);
        self::assertSame('video', $trailer['kind']);
        $version = $registry->currentVersion((string) $trailer['id']);
        self::assertSame('catpope/videos/s1-trailer-loop.1920w.mp4', $version['path'] ?? null);
        $labels = array_column($registry->renditions((int) $version['id']), 'quality', 'label');
        self::assertSame(['1080p' => 'high', '1280w' => null, '1920w' => null, '480p' => 'low', '720p' => 'medium'], (function () use ($labels) { ksort($labels); return $labels; })());
        self::assertSame(['catpope/images/3x/s1c3-key-visual.jpg', 'catpope/images/1x/s1c3-key-visual.jpg'], array_reverse(array_column($registry->renditions($this->versionId('catpope/images/s1c3-key-visual')), 'path')));

        // a new rendition appears, one disappears (out-of-band changes, e.g. cdn/sync.py)
        unlink($this->dir . '/cdn/catpope/videos/s1-trailer-loop.1280w.mp4');
        $this->put('catpope/videos/s1-trailer-loop.360p.mp4', self::MP4);
        $this->put('catpope/videos/s1-trailer-loop.480p.mp4', self::MP4 . 'changed!');
        $third = $importer->import($this->get(AssetStorages::class)->local());
        self::assertSame([0, 1, 1, 1], [$third['assets_created'], $third['renditions_added'], $third['renditions_updated'], $third['renditions_removed']]);
    }

    public function testUsageIndex(): void
    {
        $this->fixture();
        $this->scan();
        $registry = $this->get(AssetRegistry::class);
        $usages = $registry->usagesOf(['catpope/videos/s1-trailer-loop.720p.mp4', 'catpope/images/webtoon/s1c1/s1c1-slice-1.webp', 'characters/catpope/catpope_portrait.webp', 'catpope/items/vinyl.png', 'catpope/images/s1-cover-square.jpg', 'catpope/audio/album.zip']);
        $rows = array_map(fn($u) => [$u['path'], $u['ref'], $u['field'], $u['state']], $usages);
        self::assertContains(['catpope/videos/s1-trailer-loop.720p.mp4', 'season:catpope:1:de', '/trailer/loop', 'published'], $rows);
        self::assertContains(['catpope/images/webtoon/s1c1/s1c1-slice-1.webp', 'chapter:catpope:1:de', '/groups/0/slices/0/image', 'published'], $rows);
        self::assertContains(['catpope/images/webtoon/s1c1/s1c1-slice-1.webp', 'chapter:catpope:1:de', '/groups/0/slices/0/image', 'draft'], $rows, 'the newer working copy is indexed too');
        self::assertContains(['catpope/images/s1-cover-square.jpg', 'chapter:catpope:1:de', '/groups/0/slices/1/image', 'draft'], $rows, 'only the draft uses the jpg');
        self::assertContains(['catpope/images/webtoon/s1c1/s1c1-slice-1.webp', 'map', '/locations/0/unlock/slice', 'published'], $rows, 'the map resolves against the location\'s toon');
        self::assertContains(['characters/catpope/catpope_portrait.webp', 'chapter:catpope:1:de', '/tile', 'published'], $rows);
        self::assertContains(['catpope/items/vinyl.png', 'items.json', '/items/0/icon', 'config'], $rows);
        self::assertContains(['catpope/audio/album.zip', 'items.json', '/albums/0/download', 'config'], $rows);

        $missing = $this->get(AssetUsages::class)->missing();
        self::assertSame(['catpope/images/missing-poster.webp'], array_column($missing, 'path'), 'asset-requests are wishes, not usages');
        $orphans = array_column($this->get(AssetUsages::class)->orphans(), null, 'slug');
        self::assertTrue($orphans['catpope/videos/roar-little-kitty']['all_unused']);
        self::assertSame(['catpope/videos/s1-trailer-loop.1280w.mp4', 'catpope/videos/s1-trailer-loop.1920w.mp4'], $orphans['catpope/videos/s1-trailer-loop']['unused']);
    }

    public function testDeleteIsRefusedWhileReferenced(): void
    {
        $this->fixture();
        $this->scan();
        $service = $this->get(AssetService::class);
        $admin = $this->user('admin@example.com');
        $registry = $this->get(AssetRegistry::class);
        $trailer = $this->assetId('catpope/videos/s1-trailer-loop');
        $check = $service->deleteCheck($trailer);
        self::assertFalse($check['can_delete']);
        self::assertSame(3, $check['blocking']);
        try {
            $service->delete($admin, $trailer, true);
            self::fail('published usages always block');
        } catch (ApiError $error) {
            self::assertSame([409, 'in_use'], [$error->status, $error->errorCode]);
            self::assertCount(3, $error->extra['usages']);
        }
        self::assertFileExists($this->dir . '/cdn/catpope/videos/s1-trailer-loop.720p.mp4');

        // only a draft references the jpg cover → force deletes; unreferenced → plain delete
        $cover = $this->assetId('catpope/images/s1-cover-square');
        self::assertSame(['blocking' => 1, 'drafts' => 1], array_intersect_key($service->deleteCheck($cover), ['blocking' => 0, 'drafts' => 0]), 'the webp twin is published');
        $rlk = $this->assetId('catpope/videos/roar-little-kitty');
        $result = $service->delete($admin, $rlk);
        self::assertCount(3, $result['deleted']);
        self::assertFileDoesNotExist($this->dir . '/cdn/catpope/videos/roar-little-kitty.720p.mp4');
        self::assertSame('deleted', $registry->asset($rlk)['status'] ?? null);
        self::assertNull($registry->bySlug('catpope/videos/roar-little-kitty'));
    }

    public function testUploadReplaceRelinkRename(): void
    {
        $this->fixture();
        $this->scan();
        $service = $this->get(AssetService::class);
        $registry = $this->get(AssetRegistry::class);
        $user = $this->user('creator@example.com');

        // upload: a new asset, a new rendition of an existing one, refusals
        $new = $service->upload($user, $this->local(self::PNG), 'catpope/images/new-art.png', 'New art', ['Cover', 'cover', 'bad tag!']);
        self::assertSame(['catpope/images/new-art', 'New art', ['cover']], [$new['asset']['slug'], $new['asset']['title'], $new['asset']['tags']]);
        $rendition = $service->upload($user, $this->local(self::MP4 . 'new360'), 'catpope/videos/s1-trailer-loop.360p.mp4');
        self::assertSame('catpope/videos/s1-trailer-loop', $rendition['asset']['slug']);
        foreach ([['catpope/images/new-art.png', self::PNG, 'exists'], ['catpope/images/x.webp', self::PNG, 'content_mismatch'], ['catpope/images/x.exe', 'MZ', 'invalid_type'], ['x.png', self::PNG, 'invalid_path'], ['catpope/../x.png', self::PNG, 'invalid_path'], ['catpope/i/x.svg', '<svg onload="alert(1)"></svg>', 'unsafe_svg']] as [$path, $bytes, $code]) {
            try {
                $service->upload($user, $this->local($bytes), $path);
                self::fail("$path should be refused");
            } catch (ApiError $error) {
                self::assertSame($code, $error->errorCode, $path);
            }
        }

        // replace in place: same path, new version, the old bytes archived
        $trailer = $this->assetId('catpope/videos/s1-trailer-loop');
        $replaced = $service->replace($user, $trailer, $this->local(self::MP4 . 'better 720p'), 'catpope/videos/s1-trailer-loop.720p.mp4', 'sharper encode');
        self::assertSame(self::MP4 . 'better 720p', file_get_contents($this->dir . '/cdn/catpope/videos/s1-trailer-loop.720p.mp4'));
        self::assertSame(self::MP4 . '720p', file_get_contents($this->dir . '/cdn/' . $replaced['archived_as']), 'the old bytes are kept');
        self::assertSame([2, 'current'], [$replaced['versions'][0]['number'], $replaced['versions'][0]['status']]);
        self::assertSame('archived', $replaced['versions'][1]['status']);
        self::assertSame([], $replaced['stale_renditions'], 'a rendition replaced, not the source');
        self::assertCount(6, $replaced['versions'][0]['renditions']);
        try {
            $service->replace($user, $trailer, $this->local(self::MP4 . 'better 720p'), 'catpope/videos/s1-trailer-loop.720p.mp4');
            self::fail('identical bytes');
        } catch (ApiError $error) {
            self::assertSame('unchanged', $error->errorCode);
        }
        $this->get(AssetImporter::class)->import($this->get(AssetStorages::class)->local());
        self::assertNull($registry->bySlug('.versions/' . $trailer . '/1/s1-trailer-loop.720p'), 'archived files never become assets');

        // relink: the season's loop → the prologue's 720p (the owner: slice 1 and the trailer may share an asset)
        $relink = $service->relink($user, 'catpope/videos/s1-trailer-loop.720p.mp4', 'catpope/videos/roar-little-kitty.720p.mp4');
        self::assertSame(1, $relink['usages']);
        self::assertSame('season:catpope:1:de', $relink['content'][0]['ref']);
        $store = $this->get(EntityStore::class);
        $draft = json_decode((string) ($store->latest('season:catpope:1:de')['json'] ?? ''), true);
        self::assertSame('videos/roar-little-kitty.720p.mp4', $draft['trailer']['loop']);
        self::assertSame('videos/s1-trailer-loop.480p.mp4', $draft['trailer']['variants']['low'], 'other fields untouched');
        $published = json_decode((string) ($store->published('season:catpope:1:de')['json'] ?? ''), true);
        self::assertSame('videos/s1-trailer-loop.720p.mp4', $published['trailer']['loop'], 'published content changes only on publish');
        self::assertContains('draft', array_column($registry->usagesOf(['catpope/videos/roar-little-kitty.720p.mp4']), 'state'), 'the index follows');

        // rename: files copied to the new name, content (incl. the catalog) relinked, old files kept with the archived version
        $icon = $this->assetId('catpope/items/vinyl');
        $renamed = $service->rename($user, $icon, 'catpope/items/awakening-vinyl', true);
        self::assertSame(['catpope/items/vinyl.png' => 'catpope/items/awakening-vinyl.png'], $renamed['renamed']);
        self::assertFileExists($this->dir . '/cdn/catpope/items/awakening-vinyl.png');
        self::assertFileExists($this->dir . '/cdn/catpope/items/vinyl.png');
        self::assertStringContainsString('"catpope/items/awakening-vinyl.png"', (string) file_get_contents($this->dir . '/items.json'));
        self::assertSame('catpope/items/awakening-vinyl', $registry->asset($icon)['slug'] ?? null);
        $this->get(AssetImporter::class)->import($this->get(AssetStorages::class)->local());
        self::assertNull($registry->bySlug('catpope/items/vinyl'), 'the old file stays attached to the archived version');
    }

    public function testJobsQueueAndValidate(): void
    {
        $this->fixture();
        $this->scan();
        $registry = $this->get(AssetRegistry::class);
        $jobs = $this->get(AssetJobs::class);
        $video = $this->assetId('catpope/videos/roar-little-kitty');
        $job = $jobs->enqueue($video, 'video-renditions', ['levels' => [480]], null);
        self::assertSame(['video-renditions', 'queued'], [$job['kind'], $job['status']]);
        self::assertSame($job['id'], $jobs->next());
        try {
            $jobs->enqueue($this->assetId('catpope/items/vinyl'), 'video-renditions');
            self::fail('a video job for an image');
        } catch (ApiError $error) {
            self::assertSame('invalid_kind', $error->errorCode);
        }
        self::assertSame('catpope/videos/x.720p.mp4', AssetJobs::outputPath('catpope/videos/x', '720p', 'mp4'));
    }

    public function testLayoutPlan(): void
    {
        $layout = $this->get(AssetLayout::class);
        $seasons = ['catpope' => [0 => 1, 1 => 1, 2 => 1]];
        $chapter0 = [['source' => 'content', 'ref' => 'chapter:catpope:0:de', 'state' => 'published']];
        $season1 = [['source' => 'content', 'ref' => 'season:catpope:1:de', 'state' => 'published'], ['source' => 'catalog', 'ref' => 'items.json', 'state' => 'config']];
        foreach ([
            ['catpope/images/webtoon/s1c2/s1c2-slice-14.webp', [], 'catpope/s1/c2/s1c2-slice-14.webp'],
            ['catpope/images/3x/s1c3-key-visual.jpg', [], 'catpope/s1/c3/s1c3-key-visual.3x.jpg'],
            ['catpope/images/1x/s1c3-key-visual.jpg', [], 'catpope/s1/c3/s1c3-key-visual.jpg'],
            ['catpope/images/1x/de/s1c2-slice-16.jpg', [], 'catpope/s1/c2/de/s1c2-slice-16.jpg'],
            ['catpope/videos/s1-trailer-loop.720p.mp4', [], 'catpope/s1/s1-trailer-loop.720p.mp4'],
            ['catpope/videos/s1c1-key-visual.900w.mp4', [], 'catpope/s1/c1/s1c1-key-visual.900w.mp4'],
            ['catpope/videos/roar-little-kitty.480p.mp4', $chapter0, 'catpope/s1/c0/roar-little-kitty.480p.mp4'],
            ['catpope/audio/CatPope-Awakening-Music.zip', $season1, 'catpope/s1/CatPope-Awakening-Music.zip'],
            ['catpope/images/supporters/ske.svg', $season1, 'catpope/s1/supporters/ske.svg'],
            ['catpope/items/vinyl.png', [], 'catpope/items/vinyl.png'],
            ['catpope/images/map/terrain.svg', [], 'catpope/map/terrain.svg'],
            ['catpope/avatar.webp', [], 'catpope/avatar.webp'],
            ['characters/catpope/x.webp', [], 'characters/catpope/x.webp'],
            ['models/m/model.json', [], 'scutoons/models/m/model.json'],
        ] as [$from, $uses, $to]) {
            self::assertSame($to, $layout->target($from, $uses, $seasons)[0], $from);
        }
        self::assertSame(['', 'delete: lettered master (owner)'], $layout->target('catpope/images/webtoon/de/S1C1 - Slice 7.jpg', [], $seasons));
        self::assertSame(['', 'delete: unused, no place (owner)'], $layout->target('catpope/images/vectors/a.svg', [], $seasons));
        self::assertSame('unplaced', $layout->target('catpope/images/vectors/a.svg', [$chapter0[0], ['source' => 'content', 'ref' => 'toon:catpope', 'state' => 'draft'], ['source' => 'content', 'ref' => 'season:catpope:2:de', 'state' => 'draft']], $seasons)[1], 'used in two seasons, no name rule');
        $plan = $layout->plan(['catpope/images/webtoon/s1c2/s1c2-slice-1.webp', 'catpope/images/s1c2-slice-1.webp', 'models/m/model.json']);
        self::assertSame(1, $plan['summary']['conflicts'], 'two files would land on one path');
        self::assertSame(0, $plan['summary']['deletes']);
        self::assertSame(['viewer/src/lib/components/camera/object-detector.ts (MOBILENET_MODEL)'], $plan['summary']['code_changes']);
    }

    public function testProbe(): void
    {
        self::assertSame(['width' => 390, 'height' => 625], MediaProbe::svgSize('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 390 625">'));
        self::assertSame(['width' => 100, 'height' => 50], MediaProbe::svgSize('<svg width="100px" height="50" viewBox="0 0 1 1">'));
        $png = $this->local(self::PNG);
        self::assertSame(2, $this->get(MediaProbe::class)->probe($png, 'image')['width']);
    }
}
