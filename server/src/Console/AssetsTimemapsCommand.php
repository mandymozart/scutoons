<?php
declare(strict_types=1);

namespace Scutoons\Console;

use Scutoons\Domain\Assets\AssetRegistry;
use Scutoons\Domain\Assets\AssetTimeMaps;
use Symfony\Component\Console\Attribute\AsCommand;
use Symfony\Component\Console\Command\Command;
use Symfony\Component\Console\Input\InputInterface;
use Symfony\Component\Console\Input\InputOption;
use Symfony\Component\Console\Output\OutputInterface;

/**
 * Measures the time map of every video rendition that has none (agents/ASSETS.md "Time maps"; needs deck-asset-timemap, TIMEMAP_PATH):
 *   assets:timemaps --dry-run             how many renditions are left, nothing is downloaded
 *   assets:timemaps [--batch=20] [--limit=N] [--after=<rendition id>]
 * Renditions go by id; a run prints the last id it reached – `--after=<it>` resumes there (a rerun without it takes up
 * every rendition still without a map, failed ones included).
 */
#[AsCommand(name: 'assets:timemaps', description: 'Measure the time map (offset, frame rate) of every video rendition that has none')]
final class AssetsTimemapsCommand extends Command
{
    public function __construct(private readonly AssetTimeMaps $timeMaps, private readonly AssetRegistry $registry)
    {
        parent::__construct();
    }

    protected function configure(): void
    {
        $this->addOption('dry-run', null, InputOption::VALUE_NONE, 'only count')
            ->addOption('batch', null, InputOption::VALUE_REQUIRED, 'renditions per batch', '20')
            ->addOption('limit', null, InputOption::VALUE_REQUIRED, 'stop after this many renditions')
            ->addOption('after', null, InputOption::VALUE_REQUIRED, 'resume behind this rendition id', '0');
    }

    protected function execute(InputInterface $input, OutputInterface $output): int
    {
        $this->registry->requireAvailable();
        $after = max(0, (int) $input->getOption('after'));
        if ($input->getOption('dry-run')) {
            $output->writeln(sprintf('%d video renditions without a time map%s; measuring with %s', $this->registry->countWithoutTimeMap($after), $after > 0 ? " (after #$after)" : '', $this->timeMaps->binary()));
            return self::SUCCESS;
        }
        $limit = $input->getOption('limit') !== null ? max(1, (int) $input->getOption('limit')) : null;
        $result = $this->timeMaps->backfill(max(1, (int) $input->getOption('batch')), $after, $limit, fn(string $line) => $output->writeln($line));
        $output->writeln(sprintf('measured %d, failed %d, %d left behind #%d', $result['measured'], $result['failed'], $result['left'], $result['last']));
        if ($result['left'] > 0) $output->writeln("resume: assets:timemaps --after={$result['last']}");
        return self::SUCCESS;
    }
}
