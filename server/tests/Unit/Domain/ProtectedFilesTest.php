<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Catalog\Catalog;
use Scutoons\Domain\Catalog\CatalogAdmin;
use Scutoons\Domain\Catalog\ProtectedFiles;
use Scutoons\Domain\Content\CollagePresets;
use Scutoons\Domain\Content\EntityKey;
use Scutoons\Domain\Content\EntityStore;
use Scutoons\Domain\Inventory\InventoryService;
use Scutoons\Tests\TestCase;

/**
 * Protected score files (agents/research/SCORES.md §5): signed bunny.net URLs, the MusicXML for anyone, the PDF only for
 * a signed-in owner, the public catalog never naming the paths; the collage pieces placed on a file come with its
 * MusicXML (agents/research/COLLAGE.md §4.1), custom presets baked (§12.3).
 */
final class ProtectedFilesTest extends TestCase
{
    private bool $signing = true;

    /** @return array<string, string> */
    protected function settings(): array
    {
        return ($this->signing ? ['PROTECTED_CDN_URL' => 'https://protected.example.net/', 'BUNNY_PROTECTED_CDN_KEY' => 'test-key'] : []) + parent::settings();
    }

    protected function setUp(): void
    {
        parent::setUp();
        $this->get(CatalogAdmin::class)->put('items', 'cloud9_en', self::cloud9(), 'test');
    }

    /**
     * The test score: the full score and its choir part, each with the collage given (none = no key)
     * @param array<string, mixed>|null $collage
     * @param array<string, mixed>|null $choirCollage
     * @return array<string, mixed>
     */
    private static function cloud9(?array $collage = null, ?array $choirCollage = null): array
    {
        $choir = ['musicxml' => 'scores/cloud9-choir.musicxml', 'pdf' => 'scores/cloud9-choir.pdf'] + ($choirCollage !== null ? ['collage' => $choirCollage] : []);
        return ['id' => 'cloud9_en', 'name' => ['en' => 'Cloud 9'], 'kind' => 'virtual', 'category' => 'score',
            'icon' => 'score.png', 'stackable' => false, 'starter' => false, 'grantable' => ['story'], 'slot' => null, 'skills' => [],
            'score' => ['language' => 'en', 'musicxml' => 'scores/cloud 9-en.musicxml', 'pdf' => 'scores/cloud9-en.pdf', 'parts' => ['choir' => $choir]]
                + ($collage !== null ? ['collage' => $collage] : [])];
    }

    /**
     * A score piece: a wreath at 50 % of a bar, 2 staff spaces above staff 2, 8 spaces wide, behind the notes
     * @return array<string, mixed>
     */
    private static function piece(string $id, string $bar): array
    {
        return ['id' => $id, 'src' => 'catpope/vectors/wreath.svg', 'ratio' => 1.25, 'rect' => ['x' => 50, 'y' => -2, 'width' => 8], 'anchor' => 'c',
            'preset' => 'negative', 'style' => ['blend' => 'multiply', 'shadow' => null], 'at' => ['bar' => $bar, 'staff' => 2], 'behind' => true];
    }

    public function testASignedUrlIsBunnysHs256Token(): void
    {
        $signed = $this->get(ProtectedFiles::class)->score('cloud9_en', 'musicxml', null);
        $expires = self::NOW + ProtectedFiles::TTL;
        // the reference: BunnyWay/BunnyCDN.TokenAuthentication php/url_signing.php (no parameters, no IP)
        $token = 'HS256-' . rtrim(strtr(base64_encode(hash_hmac('sha256', '/scores/cloud%209-en.musicxml' . $expires, 'test-key', true)), '+/', '-_'), '=');
        self::assertSame(['url' => "https://protected.example.net/scores/cloud%209-en.musicxml?token=$token&expires=$expires", 'expires' => $expires], $signed);
    }

    public function testThePdfIsOnlyForASignedInOwner(): void
    {
        $files = $this->get(ProtectedFiles::class);
        $user = $this->user();
        foreach ([[null, 'unauthorized'], [$user, 'not_owner']] as [$who, $code]) {
            try {
                $files->score('cloud9_en', 'pdf', $who);
                self::fail("expected $code");
            } catch (ApiError $error) {
                self::assertSame($code, $error->errorCode);
            }
        }
        $this->get(InventoryService::class)->add((string) $user['id'], 'cloud9_en', null, 1, 'story');
        self::assertStringStartsWith('https://protected.example.net/scores/cloud9-en.pdf?token=HS256-', $files->score('cloud9_en', 'pdf', $user)['url']);
        try {
            $files->score('camera', 'pdf', $user);
            self::fail('no score expected');
        } catch (ApiError $error) {
            self::assertSame('no_score', $error->errorCode);
        }
    }

    /** one item, several parts (owner 2026-10-14): the choir arrangement next to the full score */
    public function testAPartHasItsOwnSignedUrl(): void
    {
        $files = $this->get(ProtectedFiles::class);
        self::assertStringStartsWith('https://protected.example.net/scores/cloud9-choir.musicxml?token=HS256-', $files->score('cloud9_en', 'musicxml', null, 'choir')['url']);
        try {
            $files->score('cloud9_en', 'musicxml', null, 'brass');
            self::fail('no part expected');
        } catch (ApiError $error) {
            self::assertSame('no_part', $error->errorCode);
        }
    }

    /** a physical release's audio (owner 2026-10-14): the zip for signed-in readers, a track only for owners of a carrier */
    public function testAReleaseIsSignedByTheAlbumsAccess(): void
    {
        $admin = $this->get(CatalogAdmin::class);
        $admin->put('albums', 'rel', ['id' => 'rel', 'toon' => null, 'title' => ['en' => 'Rel'], 'tracks' => [['id' => 'one', 'title' => 'One', 'src' => 'albums/rel/aaa.mp3']], 'download' => 'albums/rel/bbb.zip'], 'test');
        $admin->put('items', 'rel_vinyl', ['id' => 'rel_vinyl', 'name' => ['en' => 'Rel vinyl'], 'kind' => 'virtual', 'category' => 'media', 'icon' => 'v.png', 'stackable' => false,
            'starter' => false, 'grantable' => ['story'], 'slot' => null, 'skills' => [], 'album' => 'rel', 'format' => 'vinyl'], 'test');
        $files = $this->get(ProtectedFiles::class);
        $refused = function (callable $call): string {
            try {
                $call();
            } catch (ApiError $error) {
                return $error->errorCode;
            }
            return 'none';
        };
        self::assertSame('unauthorized', $refused(fn() => $files->album('rel', null, null)));
        $user = $this->user();
        self::assertStringStartsWith('https://protected.example.net/albums/rel/bbb.zip?token=HS256-', $files->album('rel', null, $user)['url']);
        self::assertSame('not_owner', $refused(fn() => $files->album('rel', 'one', $user)));
        $this->get(InventoryService::class)->add((string) $user['id'], 'rel_vinyl', null, 1, 'story');
        self::assertStringStartsWith('https://protected.example.net/albums/rel/aaa.mp3?token=HS256-', $files->album('rel', 'one', $user)['url']);
        self::assertSame('unknown_track', $refused(fn() => $files->album('rel', 'two', $user)));
        self::assertSame('unknown_album', $refused(fn() => $files->album('nope', null, $user)));
    }

    /** a track follows the reader's media quality (owner 2026-10-16): the level's file, else the next lower one, else src */
    public function testATrackIsSignedAtTheReadersQuality(): void
    {
        $track = ['id' => 'one', 'title' => 'One', 'src' => 'albums/rel/high.mp3', 'variants' => ['low' => 'albums/rel/low.mp3', 'medium' => 'albums/rel/mid.mp3', 'high' => 'albums/rel/high.mp3']];
        $admin = $this->get(CatalogAdmin::class);
        $admin->put('albums', 'rel', ['id' => 'rel', 'toon' => null, 'title' => ['en' => 'Rel'], 'tracks' => [$track], 'access' => ['listen' => 'members']], 'test');
        $files = $this->get(ProtectedFiles::class);
        $user = $this->user();
        self::assertStringStartsWith('https://protected.example.net/albums/rel/low.mp3?token=', $files->album('rel', 'one', $user, 'low')['url']);
        self::assertStringStartsWith('https://protected.example.net/albums/rel/mid.mp3?token=', $files->album('rel', 'one', $user, 'medium')['url']);
        self::assertStringStartsWith('https://protected.example.net/albums/rel/high.mp3?token=', $files->album('rel', 'one', $user, 'high')['url']);
        self::assertStringStartsWith('https://protected.example.net/albums/rel/high.mp3?token=', $files->album('rel', 'one', $user)['url']);
        self::assertStringStartsWith('https://protected.example.net/albums/rel/high.mp3?token=', $files->album('rel', 'one', $user, 'ultra')['url']);
        self::assertSame('albums/rel/low.mp3', ProtectedFiles::trackFile(['src' => 's.mp3', 'variants' => ['low' => 'albums/rel/low.mp3']], 'high'));
        self::assertSame('s.mp3', ProtectedFiles::trackFile(['src' => 's.mp3', 'variants' => ['high' => 'h.mp3']], 'medium'));
    }

    /** the pieces placed on a file come with its signed URL – the full score's, the part's – never the file they name */
    public function testTheMusicXmlCarriesTheCollagePlacedOnIt(): void
    {
        $full = ['file' => 'scores/cloud 9-en.musicxml', 'pieces' => [self::piece('piece-full001', '3')]];
        $choir = ['file' => 'scores/cloud9-choir.musicxml', 'pieces' => [self::piece('piece-choir01', '12'), self::piece('piece-choir02', '6a')]];
        $this->get(CatalogAdmin::class)->put('items', 'cloud9_en', self::cloud9($full, $choir), 'test');
        $files = $this->get(ProtectedFiles::class);
        $score = $files->score('cloud9_en', 'musicxml', null);
        self::assertStringStartsWith('https://protected.example.net/scores/cloud%209-en.musicxml?token=HS256-', $score['url']);
        self::assertSame(['pieces' => [self::piece('piece-full001', '3')]], $score['collage'] ?? null, 'the pieces as stored, without file');
        $part = $files->score('cloud9_en', 'musicxml', null, 'choir');
        self::assertSame(['piece-choir01', 'piece-choir02'], array_column($part['collage']['pieces'] ?? [], 'id'), "the part's own pieces");
        // the PDF carries none; the public catalog strips the collage with the paths
        $user = $this->user();
        $this->get(InventoryService::class)->add((string) $user['id'], 'cloud9_en', null, 1, 'story');
        self::assertArrayNotHasKey('collage', $files->score('cloud9_en', 'pdf', $user));
        self::assertArrayNotHasKey('collage', $files->score('cloud9_en', 'pdf', $user, 'choir'));
        $item = $this->get(Catalog::class)->item('cloud9_en') ?? [];
        self::assertSame(['language' => 'en', 'pdf' => true, 'parts' => ['choir' => ['pdf' => true]]], ProtectedFiles::publicItem($item)['score']);
        // an emptied collage answers none (the stage takes it away with the last piece – a stray empty one too)
        $this->get(CatalogAdmin::class)->put('items', 'cloud9_en', self::cloud9(['file' => 'scores/cloud 9-en.musicxml', 'pieces' => []]), 'test');
        self::assertArrayNotHasKey('collage', $files->score('cloud9_en', 'musicxml', null));
        self::assertSame([], ProtectedFiles::collagePieces(['collage' => ['pieces' => ['id' => 'not a list']]]));
    }

    /** custom collage presets arrive baked with the published `colorations` (§12.3): the drawn style, no `preset` */
    public function testTheMusicXmlsPiecesAreBakedWithThePublishedPresets(): void
    {
        $pieces = [['preset' => 'night'] + self::piece('piece-a', '3'), ['preset' => 'dawn'] + self::piece('piece-b', '4'), self::piece('piece-c', '5')];
        $this->get(CatalogAdmin::class)->put('items', 'cloud9_en', self::cloud9(['file' => 'scores/cloud 9-en.musicxml', 'pieces' => $pieces]), 'test');
        $files = $this->get(ProtectedFiles::class);
        $stored = fn() => $files->score('cloud9_en', 'musicxml', null)['collage']['pieces'] ?? null;
        self::assertSame($pieces, $stored(), 'no colorations document: the pieces as stored');

        $store = $this->get(EntityStore::class);
        $save = fn(array ...$presets) => $store->insert(EntityKey::parse(CollagePresets::KEY), (string) json_encode(['presets' => $presets]), null, 'draft', null);
        $night = ['id' => 'night', 'name' => 'Night', 'fill' => '#1a2b6d', 'blend' => 'screen', 'shadow' => ['color' => '#000', 'x' => 1, 'y' => 1, 'blur' => 2]];
        $store->setPublished(['id' => $save($night), 'entity_key' => CollagePresets::KEY]);
        $save($night, ['id' => 'dawn', 'name' => 'Dawn', 'fill' => '#ff1900']);
        $baked = $stored();
        $piece = self::piece('piece-a', '3');
        unset($piece['preset']);
        $piece['style'] = ['blend' => 'multiply', 'fill' => '#1a2b6d'];
        self::assertSame($piece, $baked[0] ?? null, "the piece's own blend and switched-off shadow over the preset's, no preset");
        self::assertSame([$pieces[1], $pieces[2]], [$baked[1] ?? null, $baked[2] ?? null], 'a draft-only preset and a code look: as stored');
        self::assertSame($pieces, ProtectedFiles::collagePieces($this->get(Catalog::class)->item('cloud9_en')['score'] ?? []), 'the catalog row is untouched');
        // the PDF answer carries no pieces
        $user = $this->user();
        $this->get(InventoryService::class)->add((string) $user['id'], 'cloud9_en', null, 1, 'story');
        self::assertArrayNotHasKey('collage', $files->score('cloud9_en', 'pdf', $user));
    }

    /** the catalog guard: the structure readers' drawing relies on (the creator's validateContainers checks the rest) */
    public function testABrokenCollageIsRefused(): void
    {
        $admin = $this->get(CatalogAdmin::class);
        $sheet = fn(array ...$pieces) => ['file' => 'scores/cloud 9-en.musicxml', 'pieces' => $pieces];
        $piece = self::piece('piece-a', '3');
        $cases = [
            [self::cloud9($sheet(['style' => ['blend' => 'glow']] + $piece)), 'score: piece 1: blend must be one of normal, multiply'],
            [self::cloud9($sheet(['at' => ['staff' => 1]] + $piece)), 'piece 1: at.bar must name the bar'],
            [self::cloud9($sheet(['at' => ['bar' => ' ']] + $piece)), 'piece 1: at.bar must name the bar'],
            [self::cloud9($sheet($piece, $piece)), 'piece 2: id "piece-a" is used twice'],
            [self::cloud9($sheet(['src' => 'catpope/vectors/wreath.txt'] + $piece)), 'piece 1: src must be a picture file'],
            [self::cloud9($sheet(['ratio' => 0] + $piece)), 'piece 1: ratio must be'],
            [self::cloud9($sheet(['rect' => ['x' => 1, 'y' => '2', 'width' => 3]] + $piece)), 'piece 1: rect must be'],
            [self::cloud9(['file' => 'cloud9.musicxml', 'pieces' => []]), 'score: collage.file must be the scores/ .musicxml path'],
            [self::cloud9(null, ['file' => 'scores/cloud9-choir.musicxml', 'pieces' => ['id' => 'x']]), 'part choir: collage.pieces must be a list'],
            // the colours reach readers' CSS as written: hex only (collage.ts PieceColor)
            [self::cloud9($sheet(['style' => ['fill' => 'url(https://x/p.png)']] + $piece)), 'piece 1: a colour is #rgb or #rrggbb'],
            [self::cloud9($sheet(['style' => ['shadow' => ['color' => '#000) url(https://x/f.svg#f)', 'x' => 0, 'y' => 0, 'blur' => 1]]] + $piece)), 'piece 1: a colour is #rgb or #rrggbb'],
            [self::cloud9($sheet(['style' => ['outline' => ['color' => "#fff\n", 'width' => 1]]] + $piece)), 'piece 1: a colour is #rgb or #rrggbb'],
            [self::cloud9($sheet(['style' => ['fill' => ['stops' => [['at' => 0, 'color' => 'red'], ['at' => 100, 'color' => '#fff']]]]] + $piece)), 'piece 1: a colour is #rgb or #rrggbb'],
            [self::cloud9($sheet(['style' => ['fill' => ['stops' => [null, ['at' => 100, 'color' => '#fff']]]]] + $piece)), 'piece 1: a gradient stop is { at, color, opacity? }'],
            [self::cloud9($sheet(['style' => ['shadow' => 5]] + $piece)), 'piece 1: shadow must be an object or null'],
        ];
        foreach ($cases as [$data, $expect]) {
            try {
                $admin->put('items', 'cloud9_en', $data, 'test');
                self::fail("expected $expect");
            } catch (ApiError $error) {
                self::assertSame([422, 'invalid_catalog'], [$error->status, $error->errorCode], $expect);
                self::assertStringContainsString($expect, $error->getMessage());
            }
        }
        // hex colours pass: a fill, a gradient, an outline, a shadow
        $hex = ['fill' => ['angle' => 90, 'stops' => [['at' => 0, 'color' => '#fff'], ['at' => 100, 'color' => '#00d1ff', 'opacity' => 0.5]]],
            'outline' => ['color' => '#FFFFFF', 'width' => 2], 'shadow' => ['color' => '#000', 'x' => 1, 'y' => 1, 'blur' => 0]];
        $admin->put('items', 'cloud9_en', self::cloud9($sheet(['style' => $hex] + $piece, ['id' => 'piece-c', 'style' => ['fill' => '#00d1ff']] + $piece)), 'test');
        // a good one passes: a page piece, an image, no style
        $admin->put('items', 'cloud9_en', self::cloud9($sheet(['id' => 'piece-b', 'src' => 'catpope/images/x.webp', 'ratio' => 0.5, 'rect' => ['x' => 0, 'y' => 0, 'width' => 100], 'at' => ['bar' => '1', 'page' => true]])), 'test');
        $score = $this->get(Catalog::class)->item('cloud9_en')['score'] ?? [];
        self::assertSame(['piece-b'], array_column(ProtectedFiles::collagePieces(is_array($score) ? $score : []), 'id'));
    }

    public function testThePublicCatalogNeverNamesThePaths(): void
    {
        $item = $this->get(Catalog::class)->item('cloud9_en') ?? [];
        self::assertSame(['language' => 'en', 'pdf' => true, 'parts' => ['choir' => ['pdf' => true]]], ProtectedFiles::publicItem($item)['score']);
        self::assertSame(['id' => 'x'], ProtectedFiles::publicItem(['id' => 'x']));
    }

    public function testWithoutAKeyTheDevCdnIsAnswered(): void
    {
        $this->signing = false;
        $this->tearDown();
        $this->setUp();
        self::assertSame('https://cdn.scutoons.com/scores/cloud9-en.pdf', $this->get(ProtectedFiles::class)->signed('scores/cloud9-en.pdf')['url']);
    }
}
