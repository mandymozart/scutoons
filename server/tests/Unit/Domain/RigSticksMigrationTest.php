<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Infrastructure\Database\MigrationRunner;
use Scutoons\Tests\TestCase;

/**
 * The rig sticks migration (#432, 2026-11-09-rig-sticks-addresses): a rig's `sticks` move from the compiled slot
 * `<track>_<node>.<port>` to the address `<track>:<node>.<port>`, split by the rig's own track list (the longest track id
 * that prefixes the stick), in catalog_rigs (member_rigs exists in no database yet, see the migration's header); a stick
 * that already holds a `:`, is "", or matches no track stays; a second run changes nothing.
 */
final class RigSticksMigrationTest extends TestCase
{
    public function testTheCoreRigsSticksAreAddressesAfterTheMigrations(): void
    {
        $data = json_decode((string) $this->db->value('SELECT data FROM catalog_rigs WHERE id = ?', ['character.catpope']), true);
        self::assertSame([['voice:voice.ratio', 'voice:voice.index'], ['voice:melody.spread', 'drone:drone.freq']], $data['sticks']);
        $burner = json_decode((string) $this->db->value('SELECT data FROM catalog_rigs WHERE id = ?', ['item.incense_burner']), true);
        self::assertArrayNotHasKey('sticks', $burner, 'a rig without sticks is left alone');
    }

    public function testTheMigrationSplitsByTheTrackListAndRunsTwice(): void
    {
        $rig = fn(array $sticks): string => json_encode([
            'subject' => 'character.test',
            'track' => [['id' => 'a', 'instrument' => 'x'], ['id' => 'a_b', 'instrument' => 'x'], ['id' => 'c', 'instrument' => 'x']],
            'sticks' => $sticks,
        ], JSON_UNESCAPED_SLASHES | JSON_THROW_ON_ERROR);
        $this->db->exec("DELETE FROM catalog_rigs WHERE id IN ('character.test', 'character.plain', 'character.half')");
        // a track id with a `_` wins by length; a moved stick, an empty one and one of no track stay
        $this->db->run('INSERT INTO catalog_rigs (id, position, data, updated_at) VALUES (?, 9, ?, UTC_TIMESTAMP())', ['character.test', $rig([['a_b_synth.cutoff', 'a_lfo.rate'], ['', 'zz_x.y']])]);
        $this->db->run('INSERT INTO catalog_rigs (id, position, data, updated_at) VALUES (?, 9, ?, UTC_TIMESTAMP())', ['character.plain', json_encode(['subject' => 'character.plain', 'track' => [['id' => 'v', 'instrument' => 'x']]])]);
        // a moved stick next to one still to move
        $this->db->run('INSERT INTO catalog_rigs (id, position, data, updated_at) VALUES (?, 9, ?, UTC_TIMESTAMP())', ['character.half', $rig([['c_osc.freq', 'c:osc.gain'], ['', '']])]);
        $statements = MigrationRunner::statements(self::root() . '/migrations/mysql/2026-11-09-rig-sticks-addresses.mysql.sql');
        self::assertCount(1, $statements);
        $sticks = fn(string $id): array => json_decode((string) $this->db->value('SELECT data FROM catalog_rigs WHERE id = ?', [$id]), true)['sticks'];
        $state = fn(): array => [$sticks('character.test'), (string) $this->db->value('SELECT data FROM catalog_rigs WHERE id = ?', ['character.plain']), $sticks('character.half')];
        $before = $state();
        foreach ($statements as $statement) $this->db->exec($statement);
        $after = $state();
        self::assertSame([['a_b:synth.cutoff', 'a:lfo.rate'], ['', 'zz_x.y']], $after[0]);
        self::assertSame($before[1], $after[1], 'a rig without sticks is untouched');
        self::assertSame([['c:osc.freq', 'c:osc.gain'], ['', '']], $after[2]);
        foreach ($statements as $statement) $this->db->exec($statement);
        self::assertSame($after, $state(), 'a second run changes nothing');
        $this->db->exec("DELETE FROM catalog_rigs WHERE id IN ('character.test', 'character.plain', 'character.half')");
    }
}
