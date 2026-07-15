#!/bin/bash
set -e

if [ -n "$POSTGRES_MULTIPLE_DATABASES" ]; then
  for db in $(echo "$POSTGRES_MULTIPLE_DATABASES" | tr ',' ' '); do
    echo "Creating additional database: $db"
    psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" <<-EOSQL
      SELECT 'CREATE DATABASE $db OWNER medtracker_auxiliary'
      WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = '$db')\gexec
      REVOKE CONNECT ON DATABASE $db FROM PUBLIC;
      GRANT CONNECT ON DATABASE $db TO medtracker_auxiliary;
EOSQL
  done
fi
