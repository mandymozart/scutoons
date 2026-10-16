<?php
declare(strict_types=1);

namespace Scutoons\Console;

use Scutoons\Domain\Push\WebPushCrypto;
use Symfony\Component\Console\Attribute\AsCommand;
use Symfony\Component\Console\Command\Command;
use Symfony\Component\Console\Input\InputInterface;
use Symfony\Component\Console\Output\OutputInterface;

/** A new VAPID key pair for Web Push (agents/research/PWA.md §2b) – printed, never stored */
#[AsCommand(name: 'push:vapid-keys', description: 'Print a new VAPID key pair for Web Push (VAPID_PUBLIC_KEY / VAPID_PRIVATE_KEY)')]
final class PushKeysCommand extends Command
{
    protected function execute(InputInterface $input, OutputInterface $output): int
    {
        $keys = WebPushCrypto::generateKeys();
        $output->writeln('VAPID_PUBLIC_KEY=' . $keys['public']);
        $output->writeln('VAPID_PRIVATE_KEY=' . $keys['private']);
        $output->writeln('');
        $output->writeln('Set both as GitHub secrets (Settings → Secrets → Actions) and redeploy the server. Keep the pair:');
        $output->writeln('a new public key makes every existing subscription invalid (readers must turn notifications on again).');
        return Command::SUCCESS;
    }
}
