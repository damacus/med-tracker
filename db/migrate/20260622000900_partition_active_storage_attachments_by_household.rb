# frozen_string_literal: true

class PartitionActiveStorageAttachmentsByHousehold < ActiveRecord::Migration[8.1]
  def change
    add_reference :active_storage_attachments, :household, foreign_key: true, index: true
    add_index :active_storage_attachments,
              %i[id household_id],
              unique: true,
              name: 'index_active_storage_attachments_on_id_and_household_id'

    reversible do |dir|
      dir.up { enable_attachment_rls }
      dir.down { disable_attachment_rls }
    end
  end

  private

  def enable_attachment_rls
    execute 'ALTER TABLE active_storage_attachments ENABLE ROW LEVEL SECURITY;'
    execute 'ALTER TABLE active_storage_attachments FORCE ROW LEVEL SECURITY;'
    execute 'DROP POLICY IF EXISTS household_tenant_isolation ON active_storage_attachments;'
    execute <<~SQL
      CREATE POLICY household_tenant_isolation ON active_storage_attachments
      USING (
        household_id IS NULL
        OR household_id = med_tracker.current_household_id()
      )
      WITH CHECK (
        household_id IS NULL
        OR household_id = med_tracker.current_household_id()
      );
    SQL
  end

  def disable_attachment_rls
    execute 'DROP POLICY IF EXISTS household_tenant_isolation ON active_storage_attachments;'
    execute 'ALTER TABLE active_storage_attachments NO FORCE ROW LEVEL SECURITY;'
    execute 'ALTER TABLE active_storage_attachments DISABLE ROW LEVEL SECURITY;'
  end
end
