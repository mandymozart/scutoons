<?php
declare(strict_types=1);

namespace Scutoons\Console;

use Scutoons\Domain\Assets\AssetJobs;
use Scutoons\Domain\Assets\AssetRegistry;
use Scutoons\Domain\Assets\AssetService;
use Symfony\Component\Console\Attribute\AsCommand;
use Symfony\Component\Console\Command\Command;
use Symfony\Component\Console\Input\InputArgument;
use Symfony\Component\Console\Input\InputInterface;
use Symfony\Component\Console\Input\InputOption;
use Symfony\Component\Console\Output\OutputInterface;

/**
 * The asset layer's CLI for work the shared host can't do (agents/ASSETS.md "Optimization"):
 *   assets:jobs                         run every queued optimization job (needs ffmpeg)
 *   assets:jobs --queue=<asset id|slug> --kind=video-renditions [--params='{"levels":[480,720]}']
 *   assets:jobs --upload=<local file> --path=<cdn path>    upload a (big) file as a new asset / rendition
 */
#[AsCommand(name: 'assets:jobs', description: 'Run queued asset optimization jobs (ffmpeg), queue one, or upload a big file')]
final class AssetsJobsCommand extends Command
{
    public function __construct(private readonly AssetJobs $jobs, private readonly AssetRegistry $registry, private readonly AssetService $service)
    {
        parent::__construct();
    }

    protected function configure(): void
    {
        $this->addArgument('id', InputArgument::OPTIONAL, 'run only this job')
            ->addOption('queue', null, InputOption::VALUE_REQUIRED, 'queue a job for this asset (id or slug)')
            ->addOption('kind', null, InputOption::VALUE_REQUIRED, 'video-renditions | image-webp | poster')
            ->addOption('params', null, InputOption::VALUE_REQUIRED, 'job params as JSON', '{}')
            ->addOption('upload', null, InputOption::VALUE_REQUIRED, 'a local file to upload')
            ->addOption('path', null, InputOption::VALUE_REQUIRED, 'its CDN path, e.g. catpope/videos/clip.1080p.mp4')
            ->addOption('list', null, InputOption::VALUE_NONE, 'list the recent jobs');
    }

    protected function execute(InputInterface $input, OutputInterface $output): int
    {
        $this->registry->requireAvailable();
        if ($input->getOption('list')) {
            foreach ($this->jobs->recent() as $job) $output->writeln(sprintf('#%d %-16s %-8s %s', $job['id'], $job['kind'], $job['status'], $job['slug']));
            return self::SUCCESS;
        }
        if (($file = $input->getOption('upload')) !== null) {
            $result = $this->service->upload(['id' => 'cli'], (string) $file, (string) $input->getOption('path'));
            $output->writeln('Uploaded ' . $result['uploaded'] . ' → asset ' . $result['asset']['id'] . ' (' . $result['asset']['slug'] . ')');
            return self::SUCCESS;
        }
        if (($target = $input->getOption('queue')) !== null) {
            $asset = $this->registry->asset((string) $target) ?? $this->registry->bySlug((string) $target);
            if (!$asset) {
                $output->writeln("Unknown asset $target");
                return self::FAILURE;
            }
            $params = json_decode((string) $input->getOption('params'), true);
            $job = $this->jobs->enqueue((string) $asset['id'], (string) $input->getOption('kind'), is_array($params) ? $params : [], 'cli');
            $output->writeln("Queued job #{$job['id']} ({$job['kind']}) for {$asset['slug']}");
            return self::SUCCESS;
        }
        $ids = $input->getArgument('id') !== null ? [(int) $input->getArgument('id')] : [];
        if (!$ids) while (($next = $this->jobs->next()) !== null && count($ids) < 100) {
            $ids[] = $next;
            $job = $this->jobs->run($next, fn(string $line) => $output->writeln("  $line"));
            $output->writeln("#{$job['id']} {$job['kind']} {$job['slug']}: {$job['status']}");
        }
        else foreach ($ids as $id) {
            $job = $this->jobs->run($id, fn(string $line) => $output->writeln("  $line"));
            $output->writeln("#{$job['id']} {$job['kind']} {$job['slug']}: {$job['status']}");
        }
        if (!$ids) $output->writeln('No queued jobs.');
        return self::SUCCESS;
    }
}
