<?php
declare(strict_types=1);

namespace Scutoons\Console;

use Scutoons\Domain\Embassy\Embassy;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Ids;
use Symfony\Component\Console\Attribute\AsCommand;
use Symfony\Component\Console\Command\Command;
use Symfony\Component\Console\Input\InputInterface;
use Symfony\Component\Console\Input\InputOption;
use Symfony\Component\Console\Output\OutputInterface;

/**
 * Clearly fake stores for the local "Where to buy" page (agents/EMBASSY.md "Stores"): two independent shops and one
 * chain, listed, carrying real product ids. Local dev database only (the container's databases are
 * named scutoons…; production's is not) – nothing of it is in a production seed. Idempotent (same name + city = skipped); `--remove` deletes them again.
 */
#[AsCommand(name: 'sample-stores', description: 'Add (or --remove) fake sample stores to the local dev database (Embassy stores, /stores page)')]
final class SampleStoresCommand extends Command
{
    private const SOURCE = 'dev sample (bin/console sample-stores)';

    /** @var list<array<string, mixed>> */
    private const STORES = [
        ['name' => 'Testladen (Beispiel)', 'type' => 'store', 'city' => 'Wien', 'country' => 'AT', 'address' => 'Beispielgasse 1, 1070 Wien',
            'website' => 'https://example.com/testladen', 'opening' => 'Di–Sa 11–19', 'independent' => 1, 'carries' => ['awakening_vinyl', 'scorpion_amulet', 'dago_heavy_t']],
        ['name' => 'Beispiel-Plattenladen', 'type' => 'record_shop', 'city' => 'Berlin', 'country' => 'DE', 'address' => 'Musterstraße 12, 10999 Berlin',
            'website' => 'https://example.com/plattenladen', 'opening' => '', 'independent' => 1, 'carries' => ['awakening_vinyl', 'awakening_test_press']],
        ['name' => 'Testkette Megastore (Beispiel)', 'type' => 'store', 'city' => 'Graz', 'country' => 'AT', 'address' => '',
            'website' => 'https://example.com/testkette', 'opening' => '', 'independent' => 0, 'carries' => ['awakening_vinyl']],
    ];

    public function __construct(private readonly Database $db, private readonly Clock $clock, private readonly Settings $settings)
    {
        parent::__construct();
    }

    protected function configure(): void
    {
        $this->addOption('remove', null, InputOption::VALUE_NONE, 'Delete the sample stores again');
    }

    protected function execute(InputInterface $input, OutputInterface $output): int
    {
        if (!str_starts_with($this->settings->get('DB_NAME'), 'scutoons')) {
            $output->writeln('<error>sample-stores is for the local dev database only (npm run dev:db).</error>');
            return self::FAILURE;
        }
        if ($input->getOption('remove')) {
            $ids = array_column($this->db->all('SELECT id FROM embassy_orgs WHERE source = ?', [self::SOURCE]), 'id');
            foreach ($ids as $id) {
                $this->db->run('DELETE FROM embassy_activities WHERE org_id = ?', [$id]);
                $this->db->run('DELETE FROM embassy_leads WHERE org_id = ?', [$id]);
                $this->db->run('DELETE FROM embassy_people WHERE org_id = ?', [$id]);
                $this->db->run('DELETE FROM embassy_orgs WHERE id = ?', [$id]);
            }
            $output->writeln('Removed ' . count($ids) . ' sample store(s).');
            return self::SUCCESS;
        }
        $added = 0;
        foreach (self::STORES as $store) {
            $key = Embassy::nameKey((string) $store['name']);
            if ($this->db->value('SELECT 1 FROM embassy_orgs WHERE name_key = ? AND city = ?', [$key, $store['city']]) !== null) continue;
            $now = $this->clock->utc();
            $this->db->run(
                'INSERT INTO embassy_orgs (id, name, type, city, country, website, email, phone, links, tags, territories, tier, source, notes,
                    do_not_contact, name_key, created_at, updated_at, created_by, listed, independent, address, opening, carries)
                 VALUES (?, ?, ?, ?, ?, ?, \'\', \'\', \'[]\', \'["sample"]\', \'[]\', NULL, ?, ?, 0, ?, ?, ?, NULL, 1, ?, ?, ?, ?)',
                [Ids::uuid7(), $store['name'], $store['type'], $store['city'], $store['country'], $store['website'], self::SOURCE,
                    'Fake sample data for local development – not a real store.', $key, $now, $now, $store['independent'],
                    $store['address'], $store['opening'], json_encode($store['carries'])],
            );
            $added++;
        }
        $output->writeln("Added $added sample store(s) (" . (count(self::STORES) - $added) . ' already there).');
        return self::SUCCESS;
    }
}
