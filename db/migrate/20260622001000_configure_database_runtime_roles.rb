# frozen_string_literal: true

class ConfigureDatabaseRuntimeRoles < ActiveRecord::Migration[8.1]
  def up
    create_roles
    grant_roles_to_login
    transfer_application_objects_to_owner
    grant_runtime_privileges
    configure_default_privileges
  end

  def down
    grant_runtime_privileges
  end

  private

  def create_roles
    execute <<~SQL
      DO $$
      BEGIN
        IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'med_tracker_owner') THEN
          CREATE ROLE med_tracker_owner NOLOGIN;
        END IF;

        IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'med_tracker_app') THEN
          CREATE ROLE med_tracker_app NOLOGIN NOSUPERUSER NOBYPASSRLS;
        END IF;
      END
      $$;
    SQL
    execute 'ALTER ROLE med_tracker_owner NOLOGIN;'
    execute 'ALTER ROLE med_tracker_app NOLOGIN NOSUPERUSER NOBYPASSRLS;'
  end

  def grant_roles_to_login
    execute <<~SQL
      DO $$
      DECLARE
        login_role text := session_user;
      BEGIN
        EXECUTE format('GRANT med_tracker_owner TO %I', login_role);
        EXECUTE format('GRANT med_tracker_app TO %I', login_role);
      END
      $$;
    SQL
  end

  def transfer_application_objects_to_owner
    execute <<~SQL
      DO $$
      DECLARE
        app_object record;
        object_type text;
      BEGIN
        FOR app_object IN
          SELECT c.relkind, n.nspname, c.relname
          FROM pg_class c
          JOIN pg_namespace n ON n.oid = c.relnamespace
          WHERE n.nspname = 'public'
            AND c.relkind IN ('r', 'p', 'v', 'm')
        LOOP
          object_type := CASE app_object.relkind
                         WHEN 'v' THEN 'VIEW'
                         WHEN 'm' THEN 'MATERIALIZED VIEW'
                         ELSE 'TABLE'
                         END;
          EXECUTE format(
            'ALTER %s %I.%I OWNER TO med_tracker_owner',
            object_type,
            app_object.nspname,
            app_object.relname
          );
        END LOOP;

        FOR app_object IN
          SELECT p.oid::regprocedure AS signature
          FROM pg_proc p
          JOIN pg_namespace n ON n.oid = p.pronamespace
          WHERE n.nspname = 'med_tracker'
        LOOP
          EXECUTE format('ALTER FUNCTION %s OWNER TO med_tracker_owner', app_object.signature);
        END LOOP;
      END
      $$;
    SQL
  end

  def grant_runtime_privileges
    database_name = quote_table_name(select_value('SELECT current_database()'))

    execute "GRANT CONNECT ON DATABASE #{database_name} TO med_tracker_owner, med_tracker_app;"
    execute 'GRANT USAGE, CREATE ON SCHEMA public TO med_tracker_owner;'
    execute 'GRANT USAGE ON SCHEMA public TO med_tracker_app;'
    execute 'GRANT USAGE ON SCHEMA med_tracker TO med_tracker_owner, med_tracker_app;'
    execute 'GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO med_tracker_app;'
    execute 'GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO med_tracker_owner;'
    execute 'GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO med_tracker_app;'
    execute 'GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO med_tracker_owner;'
    execute 'GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA med_tracker TO med_tracker_owner, med_tracker_app;'
  end

  def configure_default_privileges
    execute <<~SQL
      ALTER DEFAULT PRIVILEGES FOR ROLE med_tracker_owner IN SCHEMA public
      GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO med_tracker_app;
    SQL
    execute <<~SQL
      ALTER DEFAULT PRIVILEGES FOR ROLE med_tracker_owner IN SCHEMA public
      GRANT USAGE, SELECT ON SEQUENCES TO med_tracker_app;
    SQL
    execute <<~SQL
      ALTER DEFAULT PRIVILEGES FOR ROLE med_tracker_owner IN SCHEMA med_tracker
      GRANT EXECUTE ON FUNCTIONS TO med_tracker_app;
    SQL
  end
end
