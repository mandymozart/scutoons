<?php

header('Content-Type: application/json');

$toons = json_decode(file_get_contents(__DIR__ . '/toons.json'), true);

http_response_code(200);
echo json_encode($toons);   
