#!/usr/bin/env fish

if test (count $argv) -ne 1
    echo "Usage: production_medication_search_smoke.fish IMAGE_REF" >&2
    exit 2
end

for command_name in docker curl jq openssl
    if not command -q $command_name
        echo "Required command is unavailable: $command_name" >&2
        exit 2
    end
end

set -g MEDICATION_SEARCH_SMOKE_IMAGE $argv[1]
set -g MEDICATION_SEARCH_SMOKE_ROOT (realpath (dirname (status filename))/..)
set -g MEDICATION_SEARCH_SMOKE_TMP (mktemp -d)
set -g MEDICATION_SEARCH_SMOKE_SUFFIX (random)
set -g MEDICATION_SEARCH_SMOKE_PREFIX "medication-search-smoke-$MEDICATION_SEARCH_SMOKE_SUFFIX"
set -g MEDICATION_SEARCH_SMOKE_NETWORK "$MEDICATION_SEARCH_SMOKE_PREFIX-network"
set -g MEDICATION_SEARCH_SMOKE_DB "$MEDICATION_SEARCH_SMOKE_PREFIX-db"
set -g MEDICATION_SEARCH_SMOKE_NHS "$MEDICATION_SEARCH_SMOKE_PREFIX-nhs"
set -g MEDICATION_SEARCH_SMOKE_STORAGE "$MEDICATION_SEARCH_SMOKE_PREFIX-storage"
set -g MEDICATION_SEARCH_SMOKE_NORMAL "$MEDICATION_SEARCH_SMOKE_PREFIX-normal"
set -g MEDICATION_SEARCH_SMOKE_UNAVAILABLE "$MEDICATION_SEARCH_SMOKE_PREFIX-unavailable"

function cleanup_medication_search_smoke --on-event fish_exit
    docker rm -f \
        $MEDICATION_SEARCH_SMOKE_NORMAL \
        $MEDICATION_SEARCH_SMOKE_UNAVAILABLE \
        $MEDICATION_SEARCH_SMOKE_NHS \
        $MEDICATION_SEARCH_SMOKE_DB >/dev/null 2>&1
    docker network rm $MEDICATION_SEARCH_SMOKE_NETWORK >/dev/null 2>&1
    docker volume rm $MEDICATION_SEARCH_SMOKE_STORAGE >/dev/null 2>&1
    if test -d $MEDICATION_SEARCH_SMOKE_TMP
        and string match -qr '^/(tmp|private/tmp|private/var/folders)/' $MEDICATION_SEARCH_SMOKE_TMP
        rm -rf -- $MEDICATION_SEARCH_SMOKE_TMP
    end
end

string join \n \
    '[req]' \
    'distinguished_name = subject' \
    'x509_extensions = extensions' \
    'prompt = no' \
    '[subject]' \
    'CN = fake-nhs' \
    '[extensions]' \
    'subjectAltName = DNS:fake-nhs' \
    'basicConstraints = critical,CA:TRUE' \
    'keyUsage = critical,digitalSignature,keyEncipherment,keyCertSign' \
    >$MEDICATION_SEARCH_SMOKE_TMP/openssl.cnf

openssl req -x509 -nodes -newkey rsa:2048 -days 1 \
    -config $MEDICATION_SEARCH_SMOKE_TMP/openssl.cnf \
    -keyout $MEDICATION_SEARCH_SMOKE_TMP/fake-nhs.key \
    -out $MEDICATION_SEARCH_SMOKE_TMP/fake-nhs.crt >/dev/null 2>&1
or exit 1

string join \n \
    'events {}' \
    'http {' \
    '  server {' \
    '    listen 443 ssl;' \
    '    server_name fake-nhs;' \
    '    ssl_certificate /etc/nginx/fake-nhs.crt;' \
    '    ssl_certificate_key /etc/nginx/fake-nhs.key;' \
    '    location = /token {' \
    '      default_type application/json;' \
    '      return 200 '"'"'{"access_token":"smoke-token","expires_in":3600}'"'"';' \
    '    }' \
    '    location /fhir/ValueSet/ {' \
    '      default_type application/json;' \
    '      return 200 '"'"'{"expansion":{"contains":[{"code":"32223611000001104","display":"Paracetamol 500mg tablets","system":"https://dmd.nhs.uk","extension":[{"url":"http://hl7.org/fhir/StructureDefinition/valueset-concept-comments","valueString":"VMP"}]}]}}'"'"';' \
    '    }' \
    '  }' \
    '}' \
    >$MEDICATION_SEARCH_SMOKE_TMP/nginx.conf

docker network create $MEDICATION_SEARCH_SMOKE_NETWORK >/dev/null
or exit 1
docker volume create $MEDICATION_SEARCH_SMOKE_STORAGE >/dev/null
or exit 1

docker run --detach \
    --name $MEDICATION_SEARCH_SMOKE_DB \
    --network $MEDICATION_SEARCH_SMOKE_NETWORK \
    --env POSTGRES_USER=medtracker \
    --env POSTGRES_PASSWORD=medtracker_password \
    --env POSTGRES_DB=medtracker \
    --env POSTGRES_MULTIPLE_DATABASES=medtracker_production_queue,medtracker_production_cache,medtracker_production_cable \
    --volume "$MEDICATION_SEARCH_SMOKE_ROOT/compose/init-roles.sql:/docker-entrypoint-initdb.d/001-init-roles.sql:ro" \
    --volume "$MEDICATION_SEARCH_SMOKE_ROOT/compose/init-multiple-dbs.sh:/docker-entrypoint-initdb.d/002-init-multiple-dbs.sh:ro" \
    postgres:18-alpine >/dev/null
or exit 1

for attempt in (seq 1 60)
    if docker exec $MEDICATION_SEARCH_SMOKE_DB pg_isready -U medtracker -d medtracker >/dev/null 2>&1
        break
    end
    if test $attempt -eq 60
        docker logs $MEDICATION_SEARCH_SMOKE_DB
        echo "PostgreSQL did not become ready" >&2
        exit 1
    end
    sleep 1
end

docker run --detach \
    --name $MEDICATION_SEARCH_SMOKE_NHS \
    --network $MEDICATION_SEARCH_SMOKE_NETWORK \
    --network-alias fake-nhs \
    --volume "$MEDICATION_SEARCH_SMOKE_TMP/nginx.conf:/etc/nginx/nginx.conf:ro" \
    --volume "$MEDICATION_SEARCH_SMOKE_TMP/fake-nhs.crt:/etc/nginx/fake-nhs.crt:ro" \
    --volume "$MEDICATION_SEARCH_SMOKE_TMP/fake-nhs.key:/etc/nginx/fake-nhs.key:ro" \
    nginx:1.29-alpine >/dev/null
or exit 1

set -g MEDICATION_SEARCH_SMOKE_APP_ENV \
    --env RAILS_ENV=production \
    --env APP_URL=http://localhost \
    --env ACTIVE_STORAGE_SERVICE=persistent \
    --env ACTIVE_STORAGE_ROOT=/app/storage \
    --env DATABASE_URL=postgresql://medtracker:medtracker_password@$MEDICATION_SEARCH_SMOKE_DB:5432/medtracker \
    --env DATABASE_ROLE= \
    --env SECRET_KEY_BASE=medication-search-production-image-smoke-secret \
    --env SOLID_QUEUE_DATABASE_URL=postgresql://medtracker:medtracker_password@$MEDICATION_SEARCH_SMOKE_DB:5432/medtracker_production_queue \
    --env SOLID_CACHE_DATABASE_URL=postgresql://medtracker:medtracker_password@$MEDICATION_SEARCH_SMOKE_DB:5432/medtracker_production_cache \
    --env SOLID_CABLE_DATABASE_URL=postgresql://medtracker:medtracker_password@$MEDICATION_SEARCH_SMOKE_DB:5432/medtracker_production_cable \
    --env NHS_DMD_CLIENT_ID=smoke-client \
    --env NHS_DMD_CLIENT_SECRET=smoke-secret \
    --env SSL_CERT_FILE=/tmp/fake-nhs.crt

set -g MEDICATION_SEARCH_SMOKE_CERT_MOUNT \
    --volume "$MEDICATION_SEARCH_SMOKE_STORAGE:/app/storage" \
    --volume "$MEDICATION_SEARCH_SMOKE_TMP/fake-nhs.crt:/tmp/fake-nhs.crt:ro"

docker run --rm \
    --network $MEDICATION_SEARCH_SMOKE_NETWORK \
    $MEDICATION_SEARCH_SMOKE_APP_ENV \
    $MEDICATION_SEARCH_SMOKE_CERT_MOUNT \
    $MEDICATION_SEARCH_SMOKE_IMAGE \
    bin/rails db:prepare
or exit 1

set -l setup_code 'result = Admin::BootstrapService.call(email: "smoke@example.test", password: "smoke-password", name: "Smoke Admin", date_of_birth: "1980-01-01"); raise result.error unless result.success?; account = Account.find_by!(email: "smoke@example.test"); household = account.households.sole; household.update!(slug: "smoke-household"); TenantContext.with(account: account, household: household, request_id: "production-image-smoke") { location = household.locations.find_by!(name: "Home"); Medication.create!(household: household, location: location, name: "Ibuprofen", dose_amount: 200, dose_unit: "mg", current_supply: 10, supply_at_last_restock: 10, reorder_threshold: 2) }; AppSettings.instance.update!(medicine_lookup_base_url: "https://fake-nhs/fhir", medicine_lookup_token_url: "https://fake-nhs/token"); raise "dm+d import data must be empty" if NhsDmdImport.exists? || NhsDmdBarcode.exists?'

docker run --rm \
    --network $MEDICATION_SEARCH_SMOKE_NETWORK \
    $MEDICATION_SEARCH_SMOKE_APP_ENV \
    $MEDICATION_SEARCH_SMOKE_CERT_MOUNT \
    $MEDICATION_SEARCH_SMOKE_IMAGE \
    bin/rails runner $setup_code
or exit 1

function run_medication_search_scenario
    set -l container_name $argv[1]
    set -l expected_guidance $argv[2]
    set -e argv[1..2]

    docker run --detach \
        --name $container_name \
        --network $MEDICATION_SEARCH_SMOKE_NETWORK \
        --publish 127.0.0.1::80 \
        $MEDICATION_SEARCH_SMOKE_APP_ENV \
        $MEDICATION_SEARCH_SMOKE_CERT_MOUNT \
        $argv \
        $MEDICATION_SEARCH_SMOKE_IMAGE >/dev/null
    or exit 1

    set -l app_port (docker port $container_name 80/tcp | string replace -r '^.*:' '')
    set -l app_url "http://localhost:$app_port"

    for attempt in (seq 1 60)
        if curl --silent --fail "$app_url/up" >/dev/null 2>&1
            break
        end
        if test $attempt -eq 60
            docker logs $container_name
            echo "Application did not become ready: $container_name" >&2
            exit 1
        end
        sleep 1
    end

    if docker inspect --format '{{range .Mounts}}{{println .Type .Destination}}{{end}}' $container_name |
            awk '$1 == "bind" && $2 ~ "^/app" { found = 1 } END { exit found }'
        true
    else
        echo "Application container has a repository bind mount" >&2
        exit 1
    end

    set -l cookie_jar "$MEDICATION_SEARCH_SMOKE_TMP/$container_name.cookies"
    set -l login_page "$MEDICATION_SEARCH_SMOKE_TMP/$container_name-login.html"
    curl --silent --show-error \
        --cookie-jar $cookie_jar \
        "$app_url/login" \
        --output $login_page
    or exit 1

    set -l authenticity_token (
        string match -r 'name="authenticity_token" value="[^"]+"' < $login_page |
            string replace 'name="authenticity_token" value="' '' |
            string replace '"' ''
    )
    if test -z "$authenticity_token"
        echo "Login authenticity token was not rendered" >&2
        exit 1
    end

    set -l login_status (
        curl --silent --show-error \
            --cookie $cookie_jar \
            --cookie-jar $cookie_jar \
            --data-urlencode "authenticity_token=$authenticity_token" \
            --data-urlencode "email=smoke@example.test" \
            --data-urlencode "password=smoke-password" \
            --output /dev/null \
            --write-out '%{http_code}' \
            "$app_url/login"
    )
    if not contains $login_status 302 303
        echo "Public login failed with HTTP $login_status" >&2
        exit 1
    end

    set -l response_file "$MEDICATION_SEARCH_SMOKE_TMP/$container_name-search.json"
    set -l search_status (
        curl --silent --show-error \
            --cookie $cookie_jar \
            --header 'Accept: application/json' \
            --get \
            --data-urlencode 'q=paracetamol' \
            --output $response_file \
            --write-out '%{http_code}' \
            "$app_url/households/smoke-household/medication-finder/search.json"
    )

    if test "$search_status" != 200
        jq . $response_file
        docker logs $container_name
        echo "Medication search returned HTTP $search_status" >&2
        exit 1
    end

    jq --exit-status \
        --arg expected_guidance $expected_guidance \
        '.results | length == 1 and
         .[0].display == "Paracetamol 500mg tablets" and
         .[0].review_prompts == [] and
         .[0].review_prompt_filter.hidden_count == 0' \
        $response_file >/dev/null
    or exit 1
    jq --exit-status \
        --arg expected_guidance $expected_guidance \
        '.review_guidance.status == $expected_guidance and
         (has("error") | not)' \
        $response_file >/dev/null
    or exit 1

    docker rm -f $container_name >/dev/null
end

run_medication_search_scenario $MEDICATION_SEARCH_SMOKE_NORMAL available
run_medication_search_scenario \
    $MEDICATION_SEARCH_SMOKE_UNAVAILABLE \
    unavailable \
    --tmpfs /app/data/medication_reviews:ro

echo "Production image medication search smoke test passed"
