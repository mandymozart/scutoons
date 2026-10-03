-- First start of the container only (docker-entrypoint-initdb.d): the database of the PHPUnit suite on MariaDB
-- (server/phpunit.mysql.xml), next to the dev database `scutoons` the image creates. scripts/dev/db.mjs repeats this
-- idempotently, so an older volume gets it too.
CREATE DATABASE IF NOT EXISTS `scutoons_test` CHARACTER SET utf8mb4 COLLATE utf8mb4_general_ci;
GRANT ALL PRIVILEGES ON `scutoons_test`.* TO 'scutoons'@'%';
