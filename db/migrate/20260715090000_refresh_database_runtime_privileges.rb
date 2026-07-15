class RefreshDatabaseRuntimePrivileges < ActiveRecord::Migration[8.1]
  def up
    execute 'GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO med_tracker_app;'
    execute 'GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO med_tracker_app;'
    execute 'GRANT USAGE ON SCHEMA med_tracker TO med_tracker_owner, med_tracker_app;'
    execute 'GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA med_tracker TO med_tracker_owner, med_tracker_app;'
    execute <<~SQL
      REVOKE INSERT, UPDATE, DELETE, TRUNCATE, REFERENCES, TRIGGER
      ON TABLE schema_migrations, ar_internal_metadata
      FROM med_tracker_app;
    SQL
    execute 'GRANT SELECT ON TABLE schema_migrations, ar_internal_metadata TO med_tracker_app;'
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

  def down
    up
  end
end
