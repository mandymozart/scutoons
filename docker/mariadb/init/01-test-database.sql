-- First start of the container only (docker-entrypoint-initdb.d): the database of the PHPUnit suite on MariaDB
-- (server/phpunit.mysql.xml), next to the dev database `scutoons` the image creates. scripts/dev/db.mjs repeats this
-- idempotently, so an older volume gets it too.
-- latin1 like production's server default: a table that does not declare utf8mb4 itself would end up latin1 here
-- (InfrastructureTest checks every column). The image's entrypoint would make it utf8mb4.
CREATE DATABASE IF NOT EXISTS `scutoons_test` CHARACTER SET latin1 COLLATE latin1_swedish_ci;
GRANT ALL PRIVILEGES ON `scutoons_test`.* TO 'scutoons'@'%';
