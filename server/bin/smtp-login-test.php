<?php
// SMTP login check from your machine (CUTOVER 6.3): connect, EHLO, AUTH LOGIN with SMTP_USER / SMTP_PASS from the
// environment, print every server reply (the password line is never printed), QUIT. Sends no mail.
//   SMTP_HOST=… SMTP_PORT=465 SMTP_SECURE=ssl SMTP_USER=… SMTP_PASS=… php -d openssl.cafile=<ca-bundle> server/bin/smtp-login-test.php
declare(strict_types=1);

$host = getenv('SMTP_HOST') ?: exit("SMTP_HOST is not set\n");
$port = (int) (getenv('SMTP_PORT') ?: 465);
$secure = getenv('SMTP_SECURE') ?: 'ssl';
$user = (string) getenv('SMTP_USER');
$pass = (string) getenv('SMTP_PASS');
printf("user: %s (%d chars), password: %d chars\n", $user, strlen($user), strlen($pass));
if ($pass !== trim($pass)) echo "warning: the password starts or ends with whitespace\n";

$socket = stream_socket_client(($secure === 'ssl' ? 'ssl://' : 'tcp://') . "$host:$port", $errno, $errstr, 20) ?: exit("connect failed: $errstr ($errno)\n");
$read = function () use ($socket): string {
    $reply = '';
    while (($line = fgets($socket, 1024)) !== false) {
        $reply .= $line;
        if (strlen($line) < 4 || $line[3] === ' ') break;
    }
    echo '< ' . str_replace("\n", "\n< ", rtrim($reply)) . "\n";
    return $reply;
};
$send = function (string $line, ?string $show = null) use ($socket, $read): string {
    echo '> ' . ($show ?? $line) . "\n";
    fwrite($socket, "$line\r\n");
    return $read();
};

$read();
$send('EHLO localhost');
if ($secure === 'tls') {
    $send('STARTTLS');
    stream_socket_enable_crypto($socket, true, STREAM_CRYPTO_METHOD_TLSv1_2_CLIENT | STREAM_CRYPTO_METHOD_TLSv1_3_CLIENT) ?: exit("STARTTLS failed\n");
    $send('EHLO localhost');
}
$send('AUTH LOGIN');
$send(base64_encode($user), '(user)');
$ok = str_starts_with($send(base64_encode($pass), '(password hidden)'), '235');
$send('QUIT');
echo $ok ? "\nLOGIN OK\n" : "\nLOGIN FAILED\n";
