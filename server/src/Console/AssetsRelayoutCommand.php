<?php
declare(strict_types=1);

namespace Scutoons\Console;

use Scutoons\Domain\Assets\AssetLayout;
use Scutoons\Domain\Assets\AssetRegistry;
use Scutoons\Domain\Assets\AssetRelayout;
use Scutoons\Infrastructure\AssetStorage\AssetStorages;
use Scutoons\Infrastructure\AssetStorage\LocalStorage;
use Scutoons\Infrastructure\Config\Settings;
use Symfony\Component\Console\Attribute\AsCommand;
use Symfony\Component\Console\Command\Command;
use Symfony\Component\Console\Input\InputInterface;
use Symfony\Component\Console\Input\InputOption;
use Symfony\Component\Console\Output\OutputInterface;

/**
 * The folder relayout (agents/ASSETS.md "Folder convention"): DRY RUN by default.
 *   --apply       copy every moving file to its new path in --source-root (adds files only, verified) and rewrite the
 *                 exact CDN paths in the repository's tracked code (content + catalog: the relink, agents/ASSETS.md)
 *   --delete-old  remove the old paths (verified copies) + the planned deletions from --source-root – a separate run,
 *                 only after the branch is merged and green
 * The report (moved / deleted / rewritten per file / for review) goes to --report.
 */
#[AsCommand(name: 'assets:relayout', description: 'Move the CDN files into the toon-hierarchy folders: copy + rewrite references (dry run by default)')]
final class AssetsRelayoutCommand extends Command
{
    public function __construct(private readonly AssetLayout $layout, private readonly AssetStorages $storages, private readonly AssetRegistry $registry, private readonly Settings $settings)
    {
        parent::__construct();
    }

    protected function configure(): void
    {
        $this->addOption('source-root', null, InputOption::VALUE_REQUIRED, 'the local CDN folder (default ASSET_LOCAL_ROOT)')
            ->addOption('apply', null, InputOption::VALUE_NONE, 'copy + rewrite (without it: dry run)')
            ->addOption('delete-old', null, InputOption::VALUE_NONE, 'delete the old paths and the planned deletions (separate, after the merge)')
            ->addOption('report', null, InputOption::VALUE_REQUIRED, 'report file', '../logs/cutover/relayout-report.json');
    }

    protected function execute(InputInterface $input, OutputInterface $output): int
    {
        $this->registry->requireAvailable();
        $root = (string) ($input->getOption('source-root') ?: '');
        $storage = $root !== '' ? new LocalStorage((string) (realpath($root) ?: $root)) : $this->storages->local();
        $apply = (bool) $input->getOption('apply');
        $deleteOld = (bool) $input->getOption('delete-old');
        $repo = (string) realpath($this->settings->root() . '/..');

        $keys = [];
        foreach ($storage->list('') as $file) $keys[] = $file->key;
        exec('git -C ' . escapeshellarg($repo) . ' ls-files', $tracked);
        $plan = $this->layout->plan($keys, AssetRelayout::codeReferences($repo, $tracked, $keys));
        $relayout = new AssetRelayout($plan['items']);
        $report = ['at' => gmdate('c'), 'source' => $storage->describe(), 'mode' => $deleteOld ? 'delete-old' : ($apply ? 'apply' : 'dry-run'), 'summary' => $plan['summary']];

        if ($deleteOld) {
            $report['delete_old'] = $relayout->deleteOld($storage, true);
            $output->writeln(sprintf('Deleted %d old files from %s; kept %d (no verified copy).', $report['delete_old']['deleted'], $storage->describe(), count($report['delete_old']['kept'])));
            $this->write($input, $report, $output);
            return $report['delete_old']['kept'] ? self::FAILURE : self::SUCCESS;
        }

        $output->writeln(($apply ? 'APPLY' : 'DRY RUN') . ' – relayout of ' . $storage->describe());
        $report['copy'] = $relayout->copy($storage, $apply);
        $output->writeln(sprintf('  copy: %d %s, %d already there, %.1f MB, %d failed', $report['copy']['copied'], $apply ? 'copied' : 'to copy', $report['copy']['identical'], $report['copy']['bytes'] / 1048576, count($report['copy']['failed'])));
        foreach ($report['copy']['failed'] as $from => $why) $output->writeln("    FAILED $from: $why");

        $rewrite = $relayout->rewrite($repo, $tracked, $apply && !$report['copy']['failed']);
        $report['rewrite'] = $rewrite;
        $refs = array_sum(array_map(fn(array $l) => array_sum(array_column($l, 'count')), $rewrite['files']));
        $output->writeln(sprintf('  references: %d in %d tracked files%s; %d conflicts; %d lines to review', $refs, count($rewrite['files']), $apply ? ' rewritten' : ' to rewrite', count($rewrite['conflicts']), count($rewrite['review'])));
        foreach ($rewrite['conflicts'] as $line) $output->writeln("    CONFLICT $line");

        $report['deletes_planned'] = $relayout->deletes();
        $output->writeln(sprintf('  planned deletions (run --delete-old after the merge): %d files + the %d old paths of moved files', count($relayout->deletes()), count($relayout->map())));
        $this->write($input, $report, $output);
        return self::SUCCESS;
    }

    /** @param array<string, mixed> $report */
    private function write(InputInterface $input, array $report, OutputInterface $output): void
    {
        $file = (string) $input->getOption('report');
        if ($file === '') return;
        if (!is_dir(dirname($file))) @mkdir(dirname($file), 0775, true);
        file_put_contents($file, json_encode($report, JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE) . "\n");
        $output->writeln("Report: $file");
    }
}
