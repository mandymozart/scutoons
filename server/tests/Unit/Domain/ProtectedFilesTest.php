<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Catalog\CatalogAdmin;
use Scutoons\Domain\Catalog\ProtectedFiles;
use Scutoons\Domain\Inventory\InventoryService;
use Scutoons\Tests\TestCase;

/**
 * Protected score files (agents/research/SCORES.md §5): signed bunny.net URLs, the MusicXML for anyone, the PDF only for
 * a signed-in owner, the public catalog never naming the paths.
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
        $this->get(CatalogAdmin::class)->put('items', 'cloud9_en', ['id' => 'cloud9_en', 'name' => ['en' => 'Cloud 9'], 'kind' => 'virtual', 'category' => 'score',
            'icon' => 'score.png', 'stackable' => false, 'starter' => false, 'grantable' => ['story'], 'slot' => null, 'skills' => [],
            'score' => ['language' => 'en', 'musicxml' => 'scores/cloud 9-en.musicxml', 'pdf' => 'scores/cloud9-en.pdf',
                'parts' => ['choir' => ['musicxml' => 'scores/cloud9-choir.musicxml', 'pdf' => 'scores/cloud9-choir.pdf']]]], 'test');
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

    public function testThePublicCatalogNeverNamesThePaths(): void
    {
        $item = $this->get(\Scutoons\Domain\Catalog\Catalog::class)->item('cloud9_en') ?? [];
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
