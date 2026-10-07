<?php
/**
 * The newsletter's unsubscribe page (Http\Controllers\NewsletterController): confirm (one button, POST to the same
 * URL) · done · invalid. Same look as the mails: ink on paper, monospace, a square frame.
 * @var callable $e  @var string $state  @var string $locale  @var ?string $email  @var string $token
 */
$t = [
    'de' => [
        'title' => 'Newsletter abmelden',
        'confirm' => 'Keine Neuigkeiten mehr von SCUTOONS an %s?',
        'button' => 'Abmelden',
        'done' => '%s bekommt keine Neuigkeiten mehr von SCUTOONS. E-Mails zu deinem Konto (z. B. Anmeldecodes) kommen weiter.',
        'again' => 'Doch wieder? In deinem Konto „Schickt mir Neuigkeiten zu neuen Veröffentlichungen“ einschalten.',
        'invalid' => 'Dieser Link ist ungültig. Antworte einfach auf die E-Mail, dann tragen wir dich aus.',
    ],
    'en' => [
        'title' => 'Unsubscribe',
        'confirm' => 'No more news from SCUTOONS to %s?',
        'button' => 'Unsubscribe',
        'done' => '%s gets no more news from SCUTOONS. Mails about your account (e.g. sign-in codes) still arrive.',
        'again' => 'Changed your mind? Switch on "Send me news about new releases" in your account.',
        'invalid' => 'This link is not valid. Just reply to the email and we will take you off the list.',
    ],
][$locale === 'de' ? 'de' : 'en'];
$mono = "'IBM Plex Mono',Menlo,Consolas,monospace";
?>
<!doctype html>
<html lang="<?= $e($locale) ?>">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="robots" content="noindex">
<title><?= $e($t['title']) ?> · SCUTOONS</title>
</head>
<body style="margin:0;padding:1.5rem 1rem;background:#ffffff;color:#000000;font-family:<?= $mono ?>;font-size:1rem;line-height:1.4">
<main style="max-width:30rem;margin:0 auto;border:2px solid #000000;padding:1.5rem">
<p style="margin:0 0 1.25rem;font-weight:700;letter-spacing:0.0625rem">SCUTOONS</p>
<?php if ($state === 'confirm'): ?>
<p style="margin:0 0 1.5rem"><?= sprintf($e($t['confirm']), '<strong>' . $e($email) . '</strong>') ?></p>
<form method="post" action="?t=<?= $e($token) ?>" style="margin:0">
<button type="submit" style="font:inherit;padding:0.75rem 2rem;background:#000000;color:#ffffff;border:0;cursor:pointer"><?= $e($t['button']) ?></button>
</form>
<?php elseif ($state === 'done'): ?>
<p style="margin:0 0 1.25rem"><?= sprintf($e($t['done']), '<strong>' . $e($email) . '</strong>') ?></p>
<p style="margin:0;color:#6b6b6b;font-size:0.8125rem"><?= $e($t['again']) ?></p>
<?php else: ?>
<p style="margin:0"><?= $e($t['invalid']) ?></p>
<?php endif ?>
</main>
</body>
</html>
