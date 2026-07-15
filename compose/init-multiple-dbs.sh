#!/bin/bash
set -e

if [ -n "$POSTGRES_MULTIPLE_DATABASES" ]; then
  for db in $(echo "$POSTGRES_MULTIPLE_DATABASES" | tr ',' ' '); do
    echo "Creating additional database: $db"
    psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "${POSTGRES_DB:-$POSTGRES_USER}" <<-EOSQL
      SELECT 'CREATE DATABASE $db OWNER medtracker_auxiliary'
      WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = '$db')\gexec
      ALTER DATABASE $db OWNER TO medtracker_auxiliary;
      REVOKE CONNECT, TEMPORARY ON DATABASE $db FROM PUBLIC;
      GRANT CONNECT ON DATABASE $db TO medtracker_auxiliary;
EOSQL
    psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$db" <<-EOSQL
      ALTER SCHEMA public OWNER TO medtracker_auxiliary;
      REVOKE CREATE ON SCHEMA public FROM PUBLIC;
      DO \$\$
      DECLARE
        owned_object record;
        object_type text;
      BEGIN
        FOR owned_object IN
          SELECT namespace.nspname, object.relname, object.relkind
          FROM pg_class object
          JOIN pg_namespace namespace ON namespace.oid = object.relnamespace
          WHERE namespace.nspname <> 'information_schema'
            AND namespace.nspname NOT LIKE 'pg_%'
            AND object.relkind IN ('r', 'p', 'S', 'v', 'm', 'f')
        LOOP
          object_type := CASE owned_object.relkind
            WHEN 'S' THEN 'SEQUENCE'
            WHEN 'v' THEN 'VIEW'
            WHEN 'm' THEN 'MATERIALIZED VIEW'
            WHEN 'f' THEN 'FOREIGN TABLE'
            ELSE 'TABLE'
          END;
          EXECUTE format(
            'ALTER %s %I.%I OWNER TO medtracker_auxiliary',
            object_type,
            owned_object.nspname,
            owned_object.relname
          );
        END LOOP;

        FOR owned_object IN
          SELECT namespace.nspname,
                 routine.proname,
                 pg_get_function_identity_arguments(routine.oid) AS arguments
          FROM pg_proc routine
          JOIN pg_namespace namespace ON namespace.oid = routine.pronamespace
          WHERE namespace.nspname <> 'information_schema'
            AND namespace.nspname NOT LIKE 'pg_%'
        LOOP
          EXECUTE format(
            'ALTER ROUTINE %I.%I(%s) OWNER TO medtracker_auxiliary',
            owned_object.nspname,
            owned_object.proname,
            owned_object.arguments
          );
        END LOOP;

        FOR owned_object IN
          SELECT namespace.nspname, type.typname, type.typtype
          FROM pg_type type
          JOIN pg_namespace namespace ON namespace.oid = type.typnamespace
          WHERE namespace.nspname <> 'information_schema'
            AND namespace.nspname NOT LIKE 'pg_%'
            AND type.typtype IN ('d', 'e')
        LOOP
          object_type := CASE owned_object.typtype WHEN 'd' THEN 'DOMAIN' ELSE 'TYPE' END;
          EXECUTE format(
            'ALTER %s %I.%I OWNER TO medtracker_auxiliary',
            object_type,
            owned_object.nspname,
            owned_object.typname
          );
        END LOOP;
      END
      \$\$;
EOSQL
  done
fi
