<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Catalog\CatalogAdmin;
use Scutoons\Domain\Inventory\Containers;
use Scutoons\Domain\Inventory\InventoryService;
use Scutoons\Tests\TestCase;

/**
 * Containers (agents/research/SCORES.md §2, §2a): moving by hand, the capacity, an overfill that warns and breaks after
 * its days (saved when emptied in time), recipes that keep a tool, take a colour and carry the contents and the age.
 */
final class ContainersTest extends TestCase
{
    private InventoryService $inventory;
    private Containers $containers;
    private string $id;

    protected function setUp(): void
    {
        parent::setUp();
        $admin = $this->get(CatalogAdmin::class);
        $colours = [['id' => 'black', 'name' => ['en' => 'Black']], ['id' => 'red', 'name' => ['en' => 'Red']]];
        $item = fn(string $id, array $extra) => $admin->put('items', $id, $extra + ['id' => $id, 'name' => ['en' => $id], 'kind' => 'virtual', 'category' => 'collectible',
            'icon' => "$id.png", 'stackable' => false, 'starter' => false, 'grantable' => ['story'], 'slot' => null, 'skills' => []], 'test');
        $item('score_a', ['category' => 'score', 'score' => ['language' => 'en', 'musicxml' => 'a.musicxml']]);
        $item('songbook_diy_broken', ['category' => 'tool', 'variants' => $colours]);
        $item('songbook_small', ['category' => 'tool', 'holds' => ['categories' => ['score'], 'max' => 2]]);
        $item('songbook_diy', ['category' => 'tool', 'variants' => $colours, 'ages' => ['tint_days' => 14, 'spots_days' => 30, 'full_days' => 90],
            'holds' => ['categories' => ['score'], 'max' => 2, 'overfill' => ['days' => 3, 'max' => 4, 'breaks_into' => 'songbook_diy_broken']]]);
        $item('songbook_diy_large', ['category' => 'tool', 'variants' => $colours, 'ages' => ['tint_days' => 14, 'spots_days' => 30, 'full_days' => 90],
            'holds' => ['categories' => ['score'], 'max' => 99]]);
        $item('drill', ['category' => 'tool']);
        $item('zip_tie', ['category' => 'consumable', 'stackable' => true]);
        $item('zip_tie_large', ['category' => 'consumable', 'stackable' => true]);
        $item('plastic_sheet', ['category' => 'consumable', 'stackable' => true, 'variants' => $colours]);
        // research/PACKING.md §3: the sound carrier's format decides; a stackable bag for the quantity rules
        $item('some_cd', ['category' => 'media', 'album' => 'awakening', 'format' => 'cd']);
        $item('cd_case', ['category' => 'bag', 'holds' => ['formats' => ['cd'], 'max' => 25]]);
        $item('dj_trolley', ['category' => 'bag', 'holds' => ['formats' => ['vinyl'], 'max' => 50]]);
        $item('tie_bag', ['category' => 'bag', 'holds' => ['items' => ['zip_tie'], 'max' => 10]]);
        $admin->put('recipes', 'songbook_diy', ['id' => 'songbook_diy', 'inputs' => [['item' => 'drill', 'qty' => 1, 'keep' => true], ['item' => 'zip_tie', 'qty' => 4]],
            'output' => ['item' => 'songbook_diy', 'qty' => 1]], 'test');
        $admin->put('recipes', 'songbook_diy_colour', ['id' => 'songbook_diy_colour', 'inputs' => [['item' => 'songbook_diy', 'qty' => 1], ['item' => 'plastic_sheet', 'qty' => 1]],
            'output' => ['item' => 'songbook_diy', 'qty' => 1, 'variant_from' => 'plastic_sheet']], 'test');
        $admin->put('recipes', 'songbook_diy_upgrade', ['id' => 'songbook_diy_upgrade', 'inputs' => [['item' => 'songbook_diy', 'qty' => 1], ['item' => 'zip_tie_large', 'qty' => 4]],
            'output' => ['item' => 'songbook_diy_large', 'qty' => 1, 'variant_from' => 'songbook_diy']], 'test');
        $this->inventory = $this->get(InventoryService::class);
        $this->containers = $this->get(Containers::class);
        $this->id = (string) $this->user()['id'];
    }

    /** @return list<string> */
    private function add(string $item, int $qty = 1, ?string $variant = null): array
    {
        return $this->inventory->add($this->id, $item, $variant, $qty, 'story');
    }

    /** @return array<string, mixed> the entry as the reader sees it */
    private function entry(string $entryId): array
    {
        foreach ($this->inventory->entries($this->id) as $e) if ($e['id'] === $entryId) return $e;
        self::fail("no entry $entryId");
    }

    /** @return array<string, mixed> the error's extra fields */
    private function refused(string $code, callable $call): array
    {
        try {
            $call();
        } catch (ApiError $error) {
            self::assertSame($code, $error->errorCode);
            return $error->extra;
        }
        self::fail("expected $code");
    }

    public function testMovingInAndOutByHandWithinTheRule(): void
    {
        [$book] = $this->add('songbook_small');
        $scores = $this->add('score_a', 3);
        [$camera] = array_column(array_filter($this->inventory->entries($this->id), fn($e) => $e['item_id'] === 'camera'), 'id');
        $this->containers->move($this->id, ['entry_id' => $scores[0], 'into' => $book]);
        self::assertSame($book, $this->entry($scores[0])['in']);
        $this->refused('does_not_fit', fn() => $this->containers->move($this->id, ['entry_id' => $camera, 'into' => $book]));
        $this->refused('not_a_container', fn() => $this->containers->move($this->id, ['entry_id' => $scores[1], 'into' => $scores[2]]));
        [$other] = $this->add('songbook_small');
        $this->refused('container_in_container', fn() => $this->containers->move($this->id, ['entry_id' => $other, 'into' => $book]));
        $this->containers->move($this->id, ['entry_id' => $scores[1], 'into' => $book]);
        self::assertSame(['max' => 2], $this->refused('container_full', fn() => $this->containers->move($this->id, ['entry_id' => $scores[2], 'into' => $book])));
        $this->containers->move($this->id, ['entry_id' => $scores[0], 'into' => null]);
        self::assertNull($this->entry($scores[0])['in'], 'taken out: back into the regular inventory');
        $stranger = (string) $this->user('other@scutoons.test')['id'];
        $this->refused('unknown_entry', fn() => $this->containers->move($stranger, ['entry_id' => $scores[2], 'into' => $book]));
    }

    public function testAnOverfilledBookWarnsBreaksAfterItsDaysOrIsSavedInTime(): void
    {
        [$book] = $this->add('songbook_diy', 1, 'red');
        $scores = $this->add('score_a', 4);
        foreach ([0, 1] as $i) $this->containers->move($this->id, ['entry_id' => $scores[$i], 'into' => $book]);
        self::assertSame(['max' => 2, 'days' => 3], $this->refused('overfill_warning', fn() => $this->containers->move($this->id, ['entry_id' => $scores[2], 'into' => $book])));
        $this->containers->move($this->id, ['entry_id' => $scores[2], 'into' => $book, 'overfill' => true]);
        self::assertNotNull($this->entry($book)['overfilled_at']);
        // saved: out again within the days
        $this->clock->advance(2 * 86400);
        $this->containers->move($this->id, ['entry_id' => $scores[2], 'into' => null]);
        self::assertNull($this->entry($book)['overfilled_at']);
        // overfilled again and left: it breaks after three days, the scores fall out, the colour stays
        $this->containers->move($this->id, ['entry_id' => $scores[2], 'into' => $book, 'overfill' => true]);
        $this->clock->advance(3 * 86400 - 60);
        self::assertSame('songbook_diy', $this->entry($book)['item_id']);
        $this->clock->advance(60);
        $broken = $this->entry($book);
        self::assertSame(['songbook_diy_broken', 'red', null], [$broken['item_id'], $broken['variant'], $broken['overfilled_at']]);
        foreach ([0, 1, 2] as $i) self::assertNull($this->entry($scores[$i])['in']);
    }

    public function testRecipesKeepTheDrillTakeTheColourAndCarryScoresAndAge(): void
    {
        $this->add('drill');
        $this->add('zip_tie', 4);
        $this->inventory->craft($this->id, 'songbook_diy');
        self::assertSame([1, 1, 0], [$this->inventory->count($this->id, 'drill'), $this->inventory->count($this->id, 'songbook_diy'), $this->inventory->count($this->id, 'zip_tie')]);
        [$book] = array_column(array_filter($this->inventory->entries($this->id), fn($e) => $e['item_id'] === 'songbook_diy'), 'id');
        $score = $this->add('score_a')[0];
        $this->containers->move($this->id, ['entry_id' => $score, 'into' => $book]);
        $made = $this->entry($book)['acquired_at'];

        $this->clock->advance(20 * 86400);
        $this->add('plastic_sheet', 1, 'black');
        [$red] = $this->add('plastic_sheet', 1, 'red');
        $this->inventory->craft($this->id, 'songbook_diy_colour', ['plastic_sheet' => $red]);
        $coloured = array_values(array_filter($this->inventory->entries($this->id), fn($e) => $e['item_id'] === 'songbook_diy'))[0];
        self::assertSame(['red', $made], [$coloured['variant'], $coloured['aged_since']], 'the picked colour, the old age');
        self::assertSame($coloured['id'], $this->entry($score)['in'], 'the score stays in the book');
        self::assertSame(1, $this->inventory->count($this->id, 'plastic_sheet'), 'the black one is left');

        $this->add('zip_tie_large', 4);
        $this->inventory->craft($this->id, 'songbook_diy_upgrade');
        $large = array_values(array_filter($this->inventory->entries($this->id), fn($e) => $e['item_id'] === 'songbook_diy_large'))[0];
        self::assertSame(['red', $made, $large['id']], [$large['variant'], $large['aged_since'], $this->entry($score)['in']]);
        self::assertSame(0, $this->inventory->count($this->id, 'songbook_diy'));
    }

    /** owner 2026-10-14: "the scores go along with the book" – what can't be given stays with the giver, loose */
    public function testAGiftedBookTakesItsScoresAlong(): void
    {
        $this->get(CatalogAdmin::class)->put('items', 'score_private', ['id' => 'score_private', 'name' => ['en' => 'p'], 'kind' => 'virtual', 'category' => 'score',
            'icon' => 'p.png', 'stackable' => false, 'starter' => false, 'grantable' => ['story'], 'slot' => null, 'skills' => [], 'giftable' => false], 'test');
        $giver = $this->db->one('SELECT * FROM users WHERE id = ?', [$this->id]) ?? [];
        $friend = $this->user('friend@example.com');
        [$book] = $this->add('songbook_diy', 1, 'red');
        $scores = [...$this->add('score_a', 2), ...$this->add('score_private')];
        foreach ($scores as $s) $this->containers->move($this->id, ['entry_id' => $s, 'into' => $book, 'overfill' => true]);
        $gift = $this->get(\Scutoons\Domain\Ownership\Gifts::class)->gift($giver, ['entry_id' => $book, 'to' => 'friend@example.com']);
        self::assertSame(['went' => 2, 'stayed' => 1], $gift['contents'] ?? null);
        $theirs = $this->inventory->entries((string) $friend['id']);
        $theirBook = array_values(array_filter($theirs, fn($e) => $e['id'] === $book))[0];
        self::assertSame(['songbook_diy', 'red'], [$theirBook['item_id'], $theirBook['variant']]);
        self::assertNotNull($theirBook['aged_since'], 'the age goes along');
        self::assertNull($theirBook['overfilled_at'], 'two scores: no longer overfilled');
        self::assertCount(2, array_filter($theirs, fn($e) => $e['in'] === $book));
        self::assertNull($this->entry($scores[2])['in'], 'the private score stays with the giver, loose');
    }

    public function testAGuestsBookKeepsItsScoresAtSignIn(): void
    {
        $this->inventory->import($this->id, ['items' => [
            ['item_id' => 'songbook_small', 'qty' => 1, 'local_id' => 'g-book'],
            ['item_id' => 'score_a', 'qty' => 1, 'local_id' => 'g-score', 'in' => 'g-book'],
            ['item_id' => 'drill', 'qty' => 1, 'local_id' => 'g-drill', 'in' => 'g-book'],
        ]]);
        $entries = $this->inventory->entries($this->id);
        $book = array_values(array_filter($entries, fn($e) => $e['item_id'] === 'songbook_small'))[0]['id'];
        $by = fn(string $item) => array_values(array_filter($entries, fn($e) => $e['item_id'] === $item))[0]['in'];
        self::assertSame([$book, null], [$by('score_a'), $by('drill')], 'the score lies in the book again; the drill does not fit');
    }

    public function testAUsedUpContainerLetsItsContentFallOut(): void
    {
        [$book] = $this->add('songbook_small');
        [$score] = $this->add('score_a');
        $this->containers->move($this->id, ['entry_id' => $score, 'into' => $book]);
        $this->inventory->remove($this->id, 'songbook_small', 1);
        self::assertNull($this->entry($score)['in']);
    }

    public function testTheFormatDecidesCdsIntoTheCdCaseLpsIntoTheTrolley(): void
    {
        [$case] = $this->add('cd_case');
        [$trolley] = $this->add('dj_trolley');
        [$cd] = $this->add('some_cd');
        [$lp] = $this->add('awakening_vinyl');
        $this->refused('does_not_fit', fn() => $this->containers->move($this->id, ['entry_id' => $lp, 'into' => $case]));
        $this->refused('does_not_fit', fn() => $this->containers->move($this->id, ['entry_id' => $cd, 'into' => $trolley]));
        $this->containers->move($this->id, ['entry_id' => $cd, 'into' => $case]);
        $this->containers->move($this->id, ['entry_id' => $lp, 'into' => $trolley]);
        self::assertSame($case, $this->entry($cd)['in']);
        self::assertSame($trolley, $this->entry($lp)['in']);
    }

    public function testPartOfAStackGoesInNeverMoreThanFitsAndMergesWhereItLands(): void
    {
        [$bag] = $this->add('tie_bag');
        [$ties] = $this->add('zip_tie', 14);
        $qtyIn = fn() => array_sum(array_map(fn($e) => $e['qty'], array_filter($this->inventory->entries($this->id), fn($e) => $e['item_id'] === 'zip_tie' && $e['in'] === $bag)));
        $loose = fn() => array_values(array_filter($this->inventory->entries($this->id), fn($e) => $e['item_id'] === 'zip_tie' && $e['in'] === null));
        // a part (held and counted): 4 go in, 10 stay
        $this->containers->move($this->id, ['entry_id' => $ties, 'into' => $bag, 'qty' => 4]);
        self::assertSame(4, $qtyIn());
        self::assertSame(10, $this->entry($ties)['qty']);
        // all of it (a tap): only what fits – 6 more, the rest stays; they join the stack in the bag
        $this->containers->move($this->id, ['entry_id' => $ties, 'into' => $bag]);
        self::assertSame(10, $qtyIn());
        self::assertCount(1, array_filter($this->inventory->entries($this->id), fn($e) => $e['item_id'] === 'zip_tie' && $e['in'] === $bag), 'one stack in the bag');
        self::assertSame(4, $this->entry($ties)['qty']);
        self::assertSame(['max' => 10], $this->refused('container_full', fn() => $this->containers->move($this->id, ['entry_id' => $ties, 'into' => $bag])));
        // a grant joins the loose stack, never the one in the bag
        $this->add('zip_tie', 2);
        self::assertSame(6, $this->entry($ties)['qty']);
        self::assertSame(10, $qtyIn());
        // three taken out join the loose stack
        [$inside] = array_column(array_filter($this->inventory->entries($this->id), fn($e) => $e['item_id'] === 'zip_tie' && $e['in'] === $bag), 'id');
        $this->containers->move($this->id, ['entry_id' => $inside, 'into' => null, 'qty' => 3]);
        self::assertSame(7, $qtyIn());
        self::assertCount(1, $loose());
        self::assertSame(9, $loose()[0]['qty']);
        $this->refused('invalid_qty', fn() => $this->containers->move($this->id, ['entry_id' => $inside, 'into' => null, 'qty' => 0]));
    }

    public function testTwoOfTheSameKindGoOntoOneFieldByHand(): void
    {
        [$a, $b, $c] = $this->add('drill', 3);
        $this->containers->stack($this->id, ['entry_id' => $a, 'onto' => $b]);
        self::assertSame(2, $this->entry($b)['qty']);
        $ids = array_column(array_filter($this->inventory->entries($this->id), fn($e) => $e['item_id'] === 'drill'), 'id');
        sort($ids);
        $want = [$b, $c];
        sort($want);
        self::assertSame($want, $ids, 'two fields left');
        $this->containers->stack($this->id, ['entry_id' => $c, 'onto' => $b]);
        self::assertSame(3, $this->entry($b)['qty']);
        // a recipe still takes one drill from the field
        $this->inventory->remove($this->id, 'drill', 1);
        self::assertSame(2, $this->entry($b)['qty']);
        $this->refused('same_entry', fn() => $this->containers->stack($this->id, ['entry_id' => $b, 'onto' => $b]));
        [$book] = $this->add('songbook_small');
        $this->refused('not_same', fn() => $this->containers->stack($this->id, ['entry_id' => $book, 'onto' => $b]));
        [$other] = $this->add('songbook_small');
        // containers keep their contents apart
        $this->refused('not_same', fn() => $this->containers->stack($this->id, ['entry_id' => $other, 'onto' => $book]));
        // not across places: one in a bag, one loose
        [$score1, $score2] = $this->add('score_a', 2);
        $this->containers->move($this->id, ['entry_id' => $score1, 'into' => $book]);
        $this->refused('not_same', fn() => $this->containers->stack($this->id, ['entry_id' => $score2, 'onto' => $score1]));
    }
}
