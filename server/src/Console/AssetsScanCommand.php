<?php
declare(strict_types=1);

namespace Scutoons\Console;

use Scutoons\Domain\Assets\AssetImporter;
use Scutoons\Domain\Assets\AssetRegistry;
use Scutoons\Domain\Assets\AssetUsages;
use Scutoons\Infrastructure\AssetStorage\AssetStorages;
use Symfony\Component\Console\Attribute\AsCommand;
use Symfony\Component\Console\Command\Command;
use Symfony\Component\Console\Input\InputInterface;
use Symfony\Component\Console\Input\InputOption;
use Symfony\Component\Console\Output\OutputInterface;

/**
 * Registers the storage's files as assets and rebuilds the usage index (agents/ASSETS.md "Import"). Reads the storage,
 * never writes to it. Safe to run any time (idempotent); the admin's Assets page has the same as "Rescan".
 */
#[AsCommand(name: 'assets:scan', description: 'Register the media files of a storage as assets and rebuild the usage index (read-only on the storage)')]
final class AssetsScanCommand extends Command
{
    public function __construct(
        private readonly AssetStorages $storages,
        private readonly AssetImporter $importer,
        private readonly AssetUsages $usages,
        private readonly AssetRegistry $registry,
    ) {
        parent::__construct();
    }

    protected function configure(): void
    {
        $this->addOption('storage', null, InputOption::VALUE_REQUIRED, 'local | bunny (default: ASSET_STORAGE)')
            ->addOption('prefix', null, InputOption::VALUE_REQUIRED, 'only this folder', '')
            ->addOption('no-probe', null, InputOption::VALUE_NONE, 'skip image sizes / video durations')
            ->addOption('hash', null, InputOption::VALUE_NONE, 'SHA-256 of every local file')
            ->addOption('usages-only', null, InputOption::VALUE_NONE, 'only rebuild the usage index')
            ->addOption('list', null, InputOption::VALUE_REQUIRED, 'print up to N missing paths / orphans', '20');
    }

    protected function execute(InputInterface $input, OutputInterface $output): int
    {
        $this->registry->requireAvailable();
        $limit = (int) $input->getOption('list');
        if (!$input->getOption('usages-only')) {
            $name = (string) ($input->getOption('storage') ?: $this->storages->primaryName());
            $storage = $this->storages->byName($name);
            $output->writeln('Scanning ' . $storage->describe() . ' …');
            $stats = $this->importer->import($storage, (string) $input->getOption('prefix'), !$input->getOption('no-probe'), (bool) $input->getOption('hash'));
            $output->writeln(sprintf('  %d files → %d assets (%d new); renditions +%d, updated %d, removed %d',
                $stats['files'], $stats['assets'], $stats['assets_created'], $stats['renditions_added'], $stats['renditions_updated'], $stats['renditions_removed']));
        }
        $usage = $this->usages->rebuild();
        $output->writeln(sprintf('Usages: %d in content (%d entities), %d in the catalog; %d point to files the registry does not know',
            $usage['content'], $usage['entities'], $usage['catalog'], $usage['missing']));
        if ($limit > 0) {
            $missing = $this->usages->missing();
            if ($missing) $output->writeln('Missing files (referenced, not on the storage):');
            foreach (array_slice($missing, 0, $limit) as $row) {
                $refs = array_unique(array_map(fn(array $u) => $u['ref'] . ' (' . $u['state'] . ')', $row['usages']));
                $output->writeln('  ' . $row['path'] . '  ← ' . implode(', ', array_slice($refs, 0, 3)) . (count($refs) > 3 ? ' …' : ''));
            }
            if (count($missing) > $limit) $output->writeln('  … ' . (count($missing) - $limit) . ' more');
            $orphans = $this->usages->orphans();
            $full = array_filter($orphans, fn(array $o) => $o['all_unused']);
            $output->writeln(sprintf('Unused: %d assets nothing references, %d more with unused renditions', count($full), count($orphans) - count($full)));
        }
        return self::SUCCESS;
    }
}
