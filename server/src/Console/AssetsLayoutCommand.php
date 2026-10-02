<?php
declare(strict_types=1);

namespace Scutoons\Console;

use Scutoons\Domain\Assets\AssetLayout;
use Scutoons\Domain\Assets\AssetRegistry;
use Scutoons\Infrastructure\AssetStorage\AssetStorages;
use Symfony\Component\Console\Attribute\AsCommand;
use Symfony\Component\Console\Command\Command;
use Symfony\Component\Console\Input\InputInterface;
use Symfony\Component\Console\Input\InputOption;
use Symfony\Component\Console\Output\OutputInterface;

/**
 * DRY RUN of the folder convention (agents/ASSETS.md "Folder convention"): old path → new path for every file of the
 * storage, why, and the uses (content / catalog) a move would rewrite. Writes nothing to the storage or the content;
 * run `assets:scan` first so the uses are current. --json writes the full mapping.
 */
#[AsCommand(name: 'assets:layout', description: 'Plan the toon-hierarchy folder layout: old → new path per file + the uses to rewrite (dry run)')]
final class AssetsLayoutCommand extends Command
{
    public function __construct(private readonly AssetLayout $layout, private readonly AssetStorages $storages, private readonly AssetRegistry $registry)
    {
        parent::__construct();
    }

    protected function configure(): void
    {
        $this->addOption('storage', null, InputOption::VALUE_REQUIRED, 'local | bunny (default: ASSET_STORAGE)')
            ->addOption('json', null, InputOption::VALUE_REQUIRED, 'write the full mapping as JSON into this file')
            ->addOption('list', null, InputOption::VALUE_REQUIRED, 'print up to N examples per reason', '6');
    }

    protected function execute(InputInterface $input, OutputInterface $output): int
    {
        $this->registry->requireAvailable();
        $storage = $this->storages->byName((string) ($input->getOption('storage') ?: $this->storages->primaryName()));
        $keys = [];
        foreach ($storage->list('') as $file) $keys[] = $file->key;
        $plan = $this->layout->plan($keys);
        $s = $plan['summary'];
        $output->writeln("DRY RUN – folder layout for {$storage->describe()}");
        $output->writeln(sprintf('  %d files: %d move, %d stay; %d uses in %d documents to rewrite; %d conflicts; %d unplaced',
            $s['files'], $s['moves'], $s['stay'], $s['uses_to_rewrite'], $s['documents_to_touch'], $s['conflicts'], $s['unplaced']));
        if ($s['unplaced']) $output->writeln("  unplaced: {$s['unplaced_unused']} of them are used by nothing (legacy – candidates to delete, owner decision)");
        foreach ($s['code_changes'] as $where) $output->writeln("  needs a code change: $where");
        $limit = (int) $input->getOption('list');
        foreach ($s['by_reason'] as $reason => $count) {
            $output->writeln(sprintf('  %-28s %4d', $reason, $count));
            foreach (array_slice(array_filter($plan['items'], fn($i) => $i['reason'] === $reason && $i['moves']), 0, $limit) as $i) {
                $output->writeln(sprintf('      %s → %s%s%s', $i['from'], $i['to'], $i['uses'] ? "  ({$i['uses']} uses)" : '', $i['conflict'] ? '  CONFLICT' : ''));
            }
        }
        if (($file = (string) $input->getOption('json')) !== '') {
            if (!is_dir(dirname($file))) @mkdir(dirname($file), 0775, true);
            file_put_contents($file, json_encode(['at' => gmdate('c'), 'storage' => $storage->describe()] + $plan, JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES) . "\n");
            $output->writeln("Mapping: $file");
        }
        return self::SUCCESS;
    }
}
