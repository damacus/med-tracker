#!/usr/bin/env fish

set -l capture_project mtloco-mail-(string lower (uuidgen))
set -l compose_file (path resolve (status dirname)/../compose.yaml)

function capture_cleanup --on-event fish_exit --inherit-variable capture_project --inherit-variable compose_file
    docker compose --env-file /dev/null -f $compose_file -p $capture_project --profile local-mail --profile fixture-storage down --volumes
end

docker compose --env-file /dev/null -f $compose_file -p $capture_project --profile local-mail --profile fixture-storage up -d --wait postgres rustfs mailpit
or exit $status

set -l database_endpoint (docker compose --env-file /dev/null -f $compose_file -p $capture_project port postgres 5432)
or exit $status
set -l storage_endpoint http://(docker compose --env-file /dev/null -f $compose_file -p $capture_project port rustfs 9000)
or exit $status
set -l smtp_endpoint (docker compose --env-file /dev/null -f $compose_file -p $capture_project port mailpit 1025)
or exit $status
set -gx MEDTRACKER_MAILPIT_URL http://(docker compose --env-file /dev/null -f $compose_file -p $capture_project port mailpit 8025)
or exit $status
for endpoint in $database_endpoint (string replace http:// '' $storage_endpoint) $smtp_endpoint (string replace http:// '' $MEDTRACKER_MAILPIT_URL)
    if not string match -rq '^127[.]0[.]0[.]1:[1-9][0-9]*$' -- $endpoint
        echo 'Expected a published loopback port for the disposable mail check' >&2
        exit 2
    end
end
set -gx MEDTRACKER_SMTP_PORT (string split : $smtp_endpoint)[-1]
set -gx DATABASE_URL postgres://medtracker:medtracker_password@$database_endpoint/medtracker_loco
set -gx MEDTRACKER_OWNED_DATABASE 1
set -gx MEDTRACKER_OWNED_STORAGE 1
set -gx ACTIVE_STORAGE_S3_ENDPOINT $storage_endpoint
set -gx ACTIVE_STORAGE_S3_BUCKET medtracker-fixture
set -gx ACTIVE_STORAGE_S3_REGION us-east-1
set -gx ACTIVE_STORAGE_S3_ACCESS_KEY_ID storage-smoke-access
set -gx ACTIVE_STORAGE_S3_SECRET_ACCESS_KEY storage-smoke-secret
set -gx ACTIVE_STORAGE_S3_FORCE_PATH_STYLE true

task foundation:storage-init STORAGE_ENDPOINT=$storage_endpoint
or exit $status
task host:test:browser TARGET=invitation_mail FILTER=invitation_smtp
exit $status
