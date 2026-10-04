<?php
declare(strict_types=1);

namespace Scutoons\Console;

use Scutoons\Domain\Assets\AssetUsages;
use Scutoons\Domain\Catalog\CatalogSeed;
use Scutoons\Infrastructure\Database\Database;
use Symfony\Component\Console\Attribute\AsCommand;
use Symfony\Component\Console\Command\Command;
use Symfony\Component\Console\Input\InputInterface;
use Symfony\Component\Console\Input\InputOption;
use Symfony\Component\Console\Output\OutputInterface;
use Throwable;

/**
 * Fills the catalog tables from the seed files server/config/*.json (agents/CATALOG.md): only empty tables (a fresh
 * local / dev database – the migration already carries the 2026-10-04 data, so this is mostly a no-op), or with
 * `--replace` every table (local only: the admin's changes in that database are overwritten; no history).
 */
#[AsCommand(name: 'catalog:seed', description: 'Fill empty catalog tables from server/config/*.json (--replace: overwrite every catalog table)')]
final class CatalogSeedCommand extends Command
{
    public function __construct(private readonly CatalogSeed $seed, private readonly Database $db, private readonly AssetUsages $usages)
    {
        parent::__construct();
    }

    protected function configure(): void
    {
        $this->addOption('replace', null, InputOption::VALUE_NONE, 'Overwrite the rows of every catalog table with the files')
            ->addOption('db', null, InputOption::VALUE_REQUIRED, 'Another SQLite file instead of the configured database');
    }

    protected function execute(InputInterface $input, OutputInterface $output): int
    {
        $file = $input->getOption('db');
        $db = is_string($file) ? Database::sqlite($file) : $this->db;
        $counts = $this->seed->seed($db, (bool) $input->getOption('replace'));
        foreach ($counts as $kind => $count) {
            $output->writeln(sprintf('  %-12s %s', $kind, $count === null ? 'kept (not empty)' : "$count rows"));
        }
        if ($db === $this->db && array_filter($counts, fn($c) => $c !== null) !== []) {
            try {
                $this->usages->reindexCatalog();
            } catch (Throwable) {
                // no asset tables yet: nothing to index
            }
        }
        return self::SUCCESS;
    }
}
