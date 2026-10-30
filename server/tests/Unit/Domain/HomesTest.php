<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Catalog\CatalogAdmin;
use Scutoons\Domain\Catalog\CatalogValidator;
use Scutoons\Domain\Characters\CharacterUnlocks;
use Scutoons\Domain\Homes\Homes;
use Scutoons\Domain\Inventory\Containers;
use Scutoons\Domain\Inventory\InventoryService;
use Scutoons\Domain\Progress\ProgressService;
use Scutoons\Tests\TestCase;

/**
 * The avatar's home (agents/research/CAVE.md): a home per character, furniture that stays, the WG of at most five, the
 * mover, a friend's visit link, workshop recipes; the avatar's map position in the merge.
 */
final class HomesTest extends TestCase
{
    private InventoryService $inventory;
    private Homes $homes;
    private string $id;

    protected function setUp(): void
    {
        parent::setUp();
        $admin = $this->get(CatalogAdmin::class);
        $item = fn(string $id, array $extra) => $admin->put('items', $id, $extra + ['id' => $id, 'name' => ['en' => $id], 'kind' => 'virtual', 'category' => 'collectible',
            'icon' => "$id.png", 'stackable' => false, 'starter' => false, 'grantable' => ['story'], 'slot' => null, 'skills' => []], 'test');
        $item('cd_rack', ['category' => 'furniture', 'furniture' => ['size' => 12, 'image' => 'scutoons/furniture/cd_rack.webp'], 'holds' => ['categories' => ['media'], 'max' => 50]]);
        $item('workbench', ['category' => 'furniture', 'furniture' => ['size' => 35, 'image' => 'scutoons/furniture/workbench.webp', 'workshop' => true]]);
        $item('some_cd', ['category' => 'media']);
        $item('songbook_diy_broken', ['category' => 'tool']);
        $item('songbook_diy', ['category' => 'tool']);
        $admin->put('recipes', 'songbook_repair', ['id' => 'songbook_repair', 'at' => 'workshop', 'inputs' => [['item' => 'songbook_diy_broken', 'qty' => 1]],
            'output' => ['item' => 'songbook_diy', 'qty' => 1]], 'test');
        $people = [];
        foreach (['catpope' => ['cathedral', null], 'yellowmouse' => ['garage', 'tatami_room'], 'toptoy' => [null, null], 'a' => ['a-flat', null], 'b' => ['b-flat', null], 'c' => ['c-flat', null], 'd' => ['d-flat', null]] as $character => [$home, $scene]) {
            $people[] = ['id' => $character, 'personas' => [['id' => 'regular', 'looks' => [['id' => 'default', 'default' => true, 'images' => (object) [], 'apose' => ['path' => "$character.webp"]]]]]]
                + ($home !== null ? ['home' => $home] : []) + ($scene !== null ? ['home_scene' => $scene] : []);
        }
        $this->storeCollection('characters', (array) json_decode((string) json_encode(['characters' => $people]), true), 'published', '2026-10-14 00:00:00');
        $this->inventory = $this->get(InventoryService::class);
        $this->homes = $this->get(Homes::class);
        $this->id = (string) $this->user()['id'];
        // catpope is every reader's default avatar; yellowmouse is unlocked too
        $this->get(CharacterUnlocks::class)->grant($this->id, 'yellowmouse', [], 'admin', null);
    }

    /** @return array<string, string> place → home id */
    private function homeIds(): array
    {
        return array_column($this->homes->forUser($this->id)['homes'], 'id', 'place');
    }

    /** @return array<string, mixed> the entry as the reader sees it */
    private function entry(string $entryId): array
    {
        foreach ($this->inventory->entries($this->id) as $e) if ($e['id'] === $entryId) return $e;
        self::fail("no entry $entryId");
    }

    private function refused(string $code, callable $call): void
    {
        try {
            $call();
        } catch (ApiError $error) {
            self::assertSame($code, $error->errorCode);
            return;
        }
        self::fail("expected $code");
    }

    public function testEveryAvatarWithAHomePlaceGetsItsHomeAndScene(): void
    {
        $list = $this->homes->forUser($this->id);
        self::assertSame(5, $list['max_residents']);
        self::assertSame(['item' => 'cucumber', 'qty' => 50], $list['mover']);
        $byPlace = array_column($list['homes'], null, 'place');
        self::assertSame(['cathedral', 'garage'], array_keys($byPlace));
        self::assertSame('cave', $byPlace['cathedral']['scene'], 'no home_scene = the cave');
        self::assertSame('tatami_room', $byPlace['garage']['scene']);
        self::assertSame(['catpope'], $byPlace['cathedral']['residents']);
        self::assertSame(['yellowmouse'], $byPlace['garage']['residents']);
        $this->homes->forUser($this->id);
        self::assertSame(2, (int) $this->db->value('SELECT COUNT(*) FROM homes WHERE user_id = ?', [$this->id]), 'made once');
        $this->refused('no_home', fn() => $this->homes->homeOf($this->id, 'toptoy'));
    }

    public function testFurnitureStandsInItsHomeAndStaysThere(): void
    {
        $homes = $this->homeIds();
        [$rack] = $this->inventory->add($this->id, 'cd_rack', null, 1, 'story');
        // carried, a rack is only carried: filled where it is set up (owner 2026-10-16)
        [$early] = $this->inventory->add($this->id, 'some_cd', null, 1, 'story');
        $this->refused('not_set_up', fn() => $this->get(Containers::class)->move($this->id, ['entry_id' => $early, 'into' => $rack]));
        $this->homes->place($this->id, ['entry_id' => $rack, 'home_id' => $homes['cathedral'], 'x' => 30, 'y' => 70.5]);
        self::assertSame(['id' => $homes['cathedral'], 'x' => 30.0, 'y' => 70.5, 'flip' => false], $this->entry($rack)['home']);
        // moved inside the home, turned
        $this->homes->place($this->id, ['entry_id' => $rack, 'home_id' => $homes['cathedral'], 'x' => 60, 'y' => 80, 'flip' => true]);
        self::assertSame(['id' => $homes['cathedral'], 'x' => 60.0, 'y' => 80.0, 'flip' => true], $this->entry($rack)['home']);
        // never into another home, never given, never counted as carried
        $this->refused('furniture_stays', fn() => $this->homes->place($this->id, ['entry_id' => $rack, 'home_id' => $homes['garage'], 'x' => 1, 'y' => 1]));
        self::assertSame(0, $this->inventory->count($this->id, 'cd_rack'));
        $this->refused('invalid_position', fn() => $this->homes->place($this->id, ['entry_id' => $rack, 'home_id' => $homes['cathedral'], 'x' => 101, 'y' => 1]));
        [$cd] = $this->inventory->add($this->id, 'some_cd', null, 1, 'story');
        $this->refused('not_furniture', fn() => $this->homes->place($this->id, ['entry_id' => $cd, 'home_id' => $homes['cathedral'], 'x' => 1, 'y' => 1]));
        // a rack in a home still holds things
        $this->get(Containers::class)->move($this->id, ['entry_id' => $cd, 'into' => $rack]);
        self::assertSame($rack, $this->entry($cd)['in']);
        $this->refused('unknown_home', fn() => $this->homes->place($this->id, ['entry_id' => $rack, 'home_id' => 'nope', 'x' => 1, 'y' => 1]));
    }

    public function testAWgOfAtMostFiveAndTheMover(): void
    {
        $homes = $this->homeIds();
        [$bench] = $this->inventory->add($this->id, 'workbench', null, 1, 'story');
        $this->homes->place($this->id, ['entry_id' => $bench, 'home_id' => $homes['garage'], 'x' => 50, 'y' => 80]);
        // yellowmouse moves in with catpope – the furniture stays behind
        $this->homes->moveIn($this->id, ['character' => 'yellowmouse', 'home_id' => $homes['cathedral']]);
        $byPlace = array_column($this->homes->forUser($this->id)['homes'], null, 'place');
        self::assertSame(['catpope', 'yellowmouse'], $byPlace['cathedral']['residents']);
        self::assertSame([], $byPlace['garage']['residents']);
        self::assertSame($homes['garage'], $this->entry($bench)['home']['id'], 'furniture stays behind');
        // back home, with the mover: 50 cucumbers, the furniture of the home it leaves goes along
        $this->homes->moveIn($this->id, ['character' => 'yellowmouse', 'home_id' => $homes['garage']]);
        $this->homes->place($this->id, ['entry_id' => $bench, 'home_id' => $homes['garage'], 'x' => 50, 'y' => 80]);
        $this->refused('cannot_afford', fn() => $this->homes->moveIn($this->id, ['character' => 'yellowmouse', 'home_id' => $homes['cathedral'], 'mover' => true]));
        $this->inventory->add($this->id, 'cucumber', null, 60, 'story');
        $this->homes->moveIn($this->id, ['character' => 'yellowmouse', 'home_id' => $homes['cathedral'], 'mover' => true]);
        self::assertSame($homes['cathedral'], $this->entry($bench)['home']['id'], 'the mover took it along');
        self::assertSame(10, $this->inventory->count($this->id, 'cucumber'));
        // only one's own avatars; five at most
        $this->refused('not_your_avatar', fn() => $this->homes->moveIn($this->id, ['character' => 'toptoy', 'home_id' => $homes['cathedral']]));
        $unlocks = $this->get(CharacterUnlocks::class);
        foreach (['a', 'b', 'c', 'd'] as $c) $unlocks->grant($this->id, $c, [], 'admin', null);
        foreach (['a', 'b', 'c'] as $c) $this->homes->moveIn($this->id, ['character' => $c, 'home_id' => $homes['cathedral']]);
        $this->refused('home_full', fn() => $this->homes->moveIn($this->id, ['character' => 'd', 'home_id' => $homes['cathedral']]));
    }

    public function testAVisitLinkShowsTheHomeAndItsWorkbench(): void
    {
        $homes = $this->homeIds();
        [$bench] = $this->inventory->add($this->id, 'workbench', null, 1, 'story');
        [$rack] = $this->inventory->add($this->id, 'cd_rack', null, 1, 'story');
        [$cd] = $this->inventory->add($this->id, 'some_cd', null, 1, 'story');
        $this->homes->place($this->id, ['entry_id' => $rack, 'home_id' => $homes['cathedral'], 'x' => 20, 'y' => 60]);
        $this->get(Containers::class)->move($this->id, ['entry_id' => $cd, 'into' => $rack]);
        $visit = $this->homes->createVisit($this->id, $homes['cathedral']);
        $seen = $this->homes->visit($visit['token']);
        self::assertSame(['place' => 'cathedral', 'scene' => 'cave'], $seen['home']);
        self::assertSame([['item_id' => 'cd_rack', 'variant' => null, 'x' => 20.0, 'y' => 60.0, 'flip' => false, 'contents' => [['item_id' => 'some_cd', 'variant' => null, 'qty' => 1]]]], $seen['furniture']);
        self::assertFalse($seen['workshop']);
        // a friend repairs their broken book at this workbench
        $this->homes->place($this->id, ['entry_id' => $bench, 'home_id' => $homes['cathedral'], 'x' => 50, 'y' => 90]);
        $friend = (string) $this->user('friend@scutoons.test')['id'];
        self::assertTrue($this->homes->workshopAt($friend, ['visit' => $visit['token']]));
        self::assertFalse($this->homes->workshopAt($friend, ['home' => $homes['cathedral']]), 'not the friend’s home');
        self::assertTrue($this->homes->workshopAt($this->id, ['home' => $homes['cathedral']]));
        $this->homes->revokeVisit($this->id, $visit['token']);
        $this->refused('unknown_visit', fn() => $this->homes->visit($visit['token']));
        self::assertFalse($this->homes->workshopAt($friend, ['visit' => $visit['token']]));
    }

    public function testTheCatalogChecksFurnitureScenesAndWorkshopRecipes(): void
    {
        $validator = $this->get(CatalogValidator::class);
        $base = ['name' => ['en' => 'x'], 'kind' => 'virtual', 'icon' => 'x.png', 'skills' => []];
        self::assertSame(['big: furniture must be { size: 5–60 (% of the room\'s width), image, workshop? }'],
            $validator->problems('items', 'big', $base + ['id' => 'big', 'category' => 'furniture', 'furniture' => ['size' => 90]]));
        self::assertSame(['bare: the category "furniture" needs furniture { size, image }'], $validator->problems('items', 'bare', $base + ['id' => 'bare', 'category' => 'furniture']));
        self::assertSame([], $validator->problems('home-scenes', 'cave', ['id' => 'cave', 'name' => ['en' => 'Cave'], 'image' => ['src' => 'a.webp', 'variants' => ['high' => 'a.4k.webp']], 'video' => ['src' => 'a.mp4']]));
        self::assertSame(['cave: video must be { src: an .mp4 / .webm path, variants? }'], $validator->problems('home-scenes', 'cave', ['id' => 'cave', 'name' => ['en' => 'Cave'], 'image' => ['src' => ''], 'video' => ['src' => 'a.gif']]));
        $set = ['id' => 'cathedral', 'name' => ['en' => 'Cathedral'], 'image' => ['src' => '']];
        self::assertSame([], $validator->problems('home-scenes', 'cathedral', $set + ['model' => ['src' => 'r/c.medium.v1.glb', 'variants' => ['low' => 'r/c.low.v1.glb']]]), 'a set's 3D room per level (#195)');
        self::assertSame(['cathedral: model must be { src: a .glb path, variants?: { <level>: a .glb path } }'], $validator->problems('home-scenes', 'cathedral', $set + ['model' => ['src' => 'r/c.glb', 'variants' => ['high' => 'r/c.gltf']]]));
        self::assertContains('at can only be "workshop"', $validator->problems('recipes', 'r', ['id' => 'r', 'at' => 'kitchen', 'inputs' => [['item' => 'songbook_diy_broken', 'qty' => 1]], 'output' => ['item' => 'songbook_diy', 'qty' => 1]]));
        self::assertSame(['cave', 'tatami_room', 'cathedral'], array_column($this->get(\Scutoons\Domain\Catalog\Catalog::class)->data()['home_scenes'], 'id'));
    }

    public function testARideCostsItsFareInCucumbers(): void
    {
        $this->refused('cannot_afford', fn() => $this->homes->ride($this->id, 'subway'));
        $this->inventory->add($this->id, 'cucumber', null, 12, 'story');
        self::assertSame(['network' => 'subway', 'paid' => 1], $this->homes->ride($this->id, 'subway'));
        self::assertSame(['network' => 'train', 'paid' => 1], $this->homes->ride($this->id, 'train'));
        self::assertSame(['network' => 'airlift', 'paid' => 10], $this->homes->ride($this->id, 'airlift'));
        self::assertSame(0, $this->inventory->count($this->id, 'cucumber'));
        $this->refused('cannot_afford', fn() => $this->homes->ride($this->id, 'tram'));
        $this->refused('unknown_network', fn() => $this->homes->ride($this->id, 'bus'));
    }

    public function testTheAvatarPositionMergesNewestFirstAndAResetSendsItHome(): void
    {
        $state = fn(int $at, string $where, ?int $reset = null) => json_decode((string) json_encode(['places' => (object) [], 'routes' => (object) [], 'positions' => (object) [], 'avatar' => ['location' => $where, 'at' => $at]] + ($reset !== null ? ['resetAt' => $reset] : [])));
        self::assertSame('station', ProgressService::mergeMapState($state(100, 'square'), $state(200, 'station'))->avatar->location);
        self::assertSame('station', ProgressService::mergeMapState($state(200, 'station'), $state(100, 'square'))->avatar->location);
        self::assertFalse(isset(ProgressService::mergeMapState($state(100, 'square'), $state(50, 'x', 150))->avatar), 'a later reset sends it home');
    }
}
