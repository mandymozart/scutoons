<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Assets\AssetImporter;
use Scutoons\Domain\Assets\AssetJobs;
use Scutoons\Domain\Assets\AssetLayout;
use Scutoons\Domain\Assets\AssetRelayout;
use Scutoons\Domain\Assets\AssetPaths;
use Scutoons\Domain\Assets\AssetRegistry;
use Scutoons\Domain\Assets\AssetService;
use Scutoons\Domain\Assets\AssetUsages;
use Scutoons\Domain\Assets\MediaProbe;
use Scutoons\Domain\Catalog\CatalogAdmin;
use Scutoons\Domain\Catalog\CatalogStore;
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
        return [
            'ASSET_STORAGE' => 'local',
            'ASSET_LOCAL_ROOT' => $this->dir . '/cdn',
            'CDN_URL' => 'http://cdn.test/',
        ] + parent::settings();
    }

    protected function setUp(): void
    {
        parent::setUp();
        // a small catalog instead of the seeded one: an item icon and an album download
        foreach (CatalogStore::KINDS as $kind => $def) $this->db->run('DELETE FROM ' . $def['table']);
        $store = $this->get(CatalogStore::class);
        $store->save('items', 'vinyl', (object) ['id' => 'vinyl', 'name' => 'Vinyl', 'kind' => 'physical', 'category' => 'media', 'icon' => 'catpope/items/vinyl.png'], 'test');
        $store->save('albums', 'a', (object) ['id' => 'a', 'title' => (object) ['en' => 'A'], 'tracks' => [], 'download' => 'catpope/audio/album.zip'], 'test');
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
        self::assertSame('catpope/images/a.webp', AssetPaths::resolve('https://scutoons.b-cdn.net/catpope/images/a.webp', '', AssetPaths::PUBLIC_BASES), 'the pull zone hostname stays valid after the domain switch');
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

        // a new rendition appears, one disappears (out-of-band changes, e.g. the bunny dashboard)
        unlink($this->dir . '/cdn/catpope/videos/s1-trailer-loop.1280w.mp4');
        $this->put('catpope/videos/s1-trailer-loop.360p.mp4', self::MP4);
        $this->put('catpope/videos/s1-trailer-loop.480p.mp4', self::MP4 . 'changed!');
        $third = $importer->import($this->get(AssetStorages::class)->local());
        self::assertSame([0, 1, 1, 1], [$third['assets_created'], $third['renditions_added'], $third['renditions_updated'], $third['renditions_removed']]);
    }

    public function testFolderFilterTakesAPathPrefix(): void
    {
        $this->put('characters/catpope/catpope_portrait-neutral.webp', self::WEBP . 'a');
        $this->put('characters/catpope-human/catpope-human_portrait-neutral.webp', self::WEBP . 'b');
        $this->put('characters/catpopes/catpopes_portrait-neutral.webp', self::WEBP . 'c');
        $this->put('characters/yellowmouse/yellowmouse_portrait-neutral.webp', self::WEBP . 'd');
        $this->put('catpope/s1/s1-cover.webp', self::WEBP . 'e');
        $this->get(AssetImporter::class)->import($this->get(AssetStorages::class)->local());
        $registry = $this->get(AssetRegistry::class);
        $slugs = fn(array $query) => array_column($registry->list($query)['items'], 'slug');

        // the toon folder never holds character images (owner 2026-10-05: "human" found nothing under "catpope")
        self::assertSame(['catpope/s1/s1-cover'], $slugs(['folder' => 'catpope']));
        // `characters/catpope` = the character's folder and its persona folders (and, as a plain prefix, catpopes/)
        self::assertSame(['characters/catpope-human/catpope-human_portrait-neutral', 'characters/catpope/catpope_portrait-neutral', 'characters/catpopes/catpopes_portrait-neutral'], $slugs(['folder' => 'characters/catpope']));
        self::assertSame(['characters/catpope-human/catpope-human_portrait-neutral'], $slugs(['folder' => 'characters/catpope', 'q' => 'human']));
        self::assertSame([], $slugs(['folder' => 'characters/cat%_']), 'LIKE wildcards are literal');
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
        self::assertContains(['catpope/items/vinyl.png', 'item:vinyl', '/icon', 'config'], $rows);
        self::assertContains(['catpope/audio/album.zip', 'album:a', '/download', 'config'], $rows);

        $missing = $this->get(AssetUsages::class)->missing();
        self::assertSame(['catpope/images/missing-poster.webp', 'catpope/vectors/vector-demon-saw-1.svg'], array_column($missing, 'path'), 'asset-requests are wishes, not usages; a file app code needs is reported missing');
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

    public function testCleanupDeletesOnlyUnusedFiles(): void
    {
        $this->fixture();
        $this->put('scutoons/models/mobilenet/model.json', '{}');
        $this->scan();
        $service = $this->get(AssetService::class);
        $registry = $this->get(AssetRegistry::class);
        $admin = $this->user('admin@example.com');
        $trailer = $this->assetId('catpope/videos/s1-trailer-loop');
        $trailerSource = (string) ($registry->currentVersion($trailer)['path'] ?? '');
        // leftovers: a rename keeps the old file with the archived version, a replace keeps the old bytes under .versions/
        $service->rename($admin, $this->assetId('catpope/items/vinyl'), 'catpope/items/awakening-vinyl', true);
        $replaced = $service->replace($admin, $trailer, $this->local(self::MP4 . 'better 480p'), 'catpope/videos/s1-trailer-loop.480p.mp4');

        $list = $service->unused();
        $items = array_column($list['items'], null, 'slug');
        $files = fn(string $slug) => array_column($items[$slug]['files'], 'what', 'path');
        self::assertArrayNotHasKey('scutoons/models/mobilenet/model', $items, 'app code references the models');
        self::assertTrue($items['catpope/videos/roar-little-kitty']['all_unused']);
        self::assertCount(3, $files('catpope/videos/roar-little-kitty'));
        $trailerFiles = $files('catpope/videos/s1-trailer-loop');
        self::assertFalse($items['catpope/videos/s1-trailer-loop']['all_unused']);
        self::assertArrayNotHasKey($trailerSource, $trailerFiles, 'the source of a used asset stays');
        self::assertSame('archive', $trailerFiles[$replaced['archived_as']]);
        self::assertSame(['catpope/items/vinyl.png' => 'old'], $files('catpope/items/awakening-vinyl'));
        self::assertSame(count($list['items']), $list['totals']['assets']);

        $rlk = $this->assetId('catpope/videos/roar-little-kitty');
        // meanwhile a draft starts using the 1280w file → skipped, not deleted
        $this->entity('chapter:catpope:2:de', ['groups' => [['slices' => [['image' => 'videos/s1-trailer-loop.1280w.mp4']]]]], false);
        $chosen = [...array_keys($files('catpope/videos/roar-little-kitty')), ...array_keys($trailerFiles), 'catpope/items/vinyl.png', 'catpope/items/vinyl.png', 'catpope/items/awakening-vinyl.png', 'scutoons/models/mobilenet/model.json'];
        $result = $service->cleanup($admin, $chosen);
        self::assertSame(1, $result['assets_deleted']);
        self::assertSame([], $result['failed']);
        $skipped = array_column($result['skipped'], 'reason', 'path');
        self::assertSame('in_use_or_unknown', $skipped['catpope/videos/s1-trailer-loop.1280w.mp4'] ?? null);
        self::assertSame('in_use_or_unknown', $skipped['catpope/items/awakening-vinyl.png'] ?? null);
        self::assertSame('in_use_or_unknown', $skipped['scutoons/models/mobilenet/model.json'] ?? null);
        self::assertFileExists($this->dir . '/cdn/catpope/videos/s1-trailer-loop.1280w.mp4');
        self::assertFileExists($this->dir . '/cdn/catpope/items/awakening-vinyl.png');
        self::assertFileExists($this->dir . '/cdn/scutoons/models/mobilenet/model.json');
        self::assertFileExists($this->dir . '/cdn/catpope/videos/s1-trailer-loop.720p.mp4');
        self::assertFileDoesNotExist($this->dir . '/cdn/catpope/videos/roar-little-kitty.720p.mp4');
        self::assertFileDoesNotExist($this->dir . '/cdn/catpope/items/vinyl.png');
        self::assertFileDoesNotExist($this->dir . '/cdn/' . $replaced['archived_as']);
        self::assertSame('deleted', $registry->asset($rlk)['status'] ?? null, 'all unused → the whole asset');

        $after = array_column($service->unused()['items'], null, 'slug');
        self::assertArrayNotHasKey('catpope/videos/roar-little-kitty', $after);
        self::assertArrayNotHasKey('catpope/items/awakening-vinyl', $after);
        self::assertNotContains('catpope/videos/s1-trailer-loop.1280w.mp4', array_column($after['catpope/videos/s1-trailer-loop']['files'] ?? [], 'path'), 'used now, so no longer offered');
        self::assertFileExists($this->dir . '/cdn/' . $trailerSource);
        $kept = array_column($registry->detail($trailer)['versions'][0]['renditions'], 'path');
        self::assertContains($trailerSource, $kept);
        self::assertContains('catpope/videos/s1-trailer-loop.1280w.mp4', $kept);
        $gone = array_diff(array_keys($trailerFiles), ['catpope/videos/s1-trailer-loop.1280w.mp4'], [$replaced['archived_as']]);
        self::assertSame([], array_values(array_intersect($gone, $kept)), 'deleted renditions leave the registry');
        foreach ($gone as $path) self::assertFileDoesNotExist($this->dir . '/cdn/' . $path);
    }

    public function testCodeReferencesMatchTheCode(): void
    {
        $root = dirname(__DIR__, 4);
        foreach (AssetUsages::CODE_REFERENCES as $reference => $file) {
            self::assertFileExists("$root/$file");
            self::assertStringContainsString($reference, (string) file_get_contents("$root/$file"), "$file no longer references $reference – update AssetUsages::CODE_REFERENCES");
        }
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
        self::assertSame('catpope/items/awakening-vinyl.png', $this->get(CatalogStore::class)->get('items', 'vinyl')['icon'] ?? null, 'the catalog row is relinked');
        $history = $this->get(CatalogAdmin::class)->history('items', 'vinyl');
        self::assertSame(['update', 'catpope/items/vinyl.png', 'catpope/items/awakening-vinyl.png'], [$history[0]['action'], $history[0]['before']->icon ?? null, $history[0]['after']->icon ?? null], 'with history');
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

    /** A language-neutral chapter / season (agents/NEUTRAL.md): its media once, under the entity itself */
    public function testNeutralDocumentsInTheIndexAndLayout(): void
    {
        $this->entity('chapter:catpope:2', ['title' => ['de' => 'Zwei', 'en' => 'Two'], 'number' => 2, 'groups' => [['slices' => [['image' => 'images/webtoon/s1c2/s1c2-slice-1.webp',
            'dialog' => [['text' => ['de' => 'images/not-a-path.webp is text', 'en' => 'x']]]]]]]]);
        $this->entity('season:catpope:3', ['title' => ['de' => 'S3'], 'sequence' => [['type' => 'chapter', 'number' => 2, 'status' => ['de' => 'published']]]]);
        $usages = $this->get(AssetUsages::class);
        $usages->reindexEntity('chapter:catpope:2');
        $rows = array_map(fn($u) => [$u['path'], $u['ref'], $u['field'], $u['state']], $this->get(AssetRegistry::class)->usagesOf(['catpope/images/webtoon/s1c2/s1c2-slice-1.webp']));
        self::assertSame([['catpope/images/webtoon/s1c2/s1c2-slice-1.webp', 'chapter:catpope:2', '/groups/0/slices/0/image', 'published']], $rows);
        self::assertSame(['catpope' => [2 => 3]], $this->get(AssetLayout::class)->chapterSeasons(), 'a neutral season places its chapters');
    }

    public function testLayoutPlan(): void
    {
        $layout = $this->get(AssetLayout::class);
        $seasons = ['catpope' => [0 => 1, 1 => 1, 2 => 1]];
        $chapter0 = [['source' => 'content', 'ref' => 'chapter:catpope:0:de', 'state' => 'published']];
        $season1 = [['source' => 'content', 'ref' => 'season:catpope:1:de', 'state' => 'published'], ['source' => 'catalog', 'ref' => 'album:awakening', 'state' => 'config']];
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
        self::assertSame('unplaced', $layout->target('catpope/images/misc/a.svg', [$chapter0[0], ['source' => 'content', 'ref' => 'toon:catpope', 'state' => 'draft'], ['source' => 'content', 'ref' => 'season:catpope:2:de', 'state' => 'draft']], $seasons)[1], 'used in two seasons, no name rule');
        $plan = $layout->plan(['catpope/images/webtoon/s1c2/s1c2-slice-1.webp', 'catpope/images/s1c2-slice-1.webp', 'models/m/model.json']);
        self::assertSame(1, $plan['summary']['conflicts'], 'two files would land on one path');
        self::assertSame(0, $plan['summary']['deletes']);
        self::assertSame(['viewer/src/lib/components/camera/object-detector.ts (MOBILENET_MODEL)'], $plan['summary']['code_changes']);
    }

    public function testRelayoutCopiesAndRewrites(): void
    {
        $this->put('catpope/videos/s1-trailer-loop.720p.mp4', self::MP4 . 'loop');
        $this->put('catpope/images/vectors/frame.svg', '<svg/>');
        $repo = $this->dir . '/repo';
        @mkdir("$repo/viewer/src", 0777, true);
        file_put_contents("$repo/viewer/src/frame.ts", "export const FRAME = 'catpope/images/vectors/frame.svg';
");
        $tracked = ['viewer/src/frame.ts'];
        $keys = ['catpope/videos/s1-trailer-loop.720p.mp4', 'catpope/images/vectors/frame.svg'];
        $code = AssetRelayout::codeReferences($repo, $tracked, $keys);
        self::assertSame(['catpope/images/vectors/frame.svg'], array_keys($code), 'code counts as a use');
        $plan = $this->get(AssetLayout::class)->plan($keys, $code);
        $relayout = new AssetRelayout($plan['items']);
        self::assertEqualsCanonicalizing(['catpope/videos/s1-trailer-loop.720p.mp4' => 'catpope/s1/s1-trailer-loop.720p.mp4', 'catpope/images/vectors/frame.svg' => 'catpope/vectors/frame.svg'], $relayout->map());

        $storage = $this->get(AssetStorages::class)->local();
        self::assertSame(2, $relayout->copy($storage, false)['copied'], 'dry run counts, copies nothing');
        self::assertFileDoesNotExist($this->dir . '/cdn/catpope/s1/s1-trailer-loop.720p.mp4');
        $copy = $relayout->copy($storage, true);
        self::assertSame([2, []], [$copy['copied'], $copy['failed']]);
        self::assertFileExists($this->dir . '/cdn/catpope/videos/s1-trailer-loop.720p.mp4', 'copy, never move');
        self::assertSame(2, $relayout->copy($storage, true)['identical']);

        $rewrite = $relayout->rewrite($repo, $tracked, true);
        self::assertStringContainsString("'catpope/vectors/frame.svg'", (string) file_get_contents("$repo/viewer/src/frame.ts"));
        self::assertCount(1, $rewrite['files']);

        $deleted = $relayout->deleteOld($storage, true);
        self::assertSame(2, $deleted['deleted']);
        self::assertFileDoesNotExist($this->dir . '/cdn/catpope/videos/s1-trailer-loop.720p.mp4');
        self::assertFileExists($this->dir . '/cdn/catpope/s1/s1-trailer-loop.720p.mp4');
    }

    public function testProbe(): void
    {
        self::assertSame(['width' => 390, 'height' => 625], MediaProbe::svgSize('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 390 625">'));
        self::assertSame(['width' => 100, 'height' => 50], MediaProbe::svgSize('<svg width="100px" height="50" viewBox="0 0 1 1">'));
        $png = $this->local(self::PNG);
        self::assertSame(2, $this->get(MediaProbe::class)->probe($png, 'image')['width']);
    }
}
