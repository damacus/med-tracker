# frozen_string_literal: true

class DropAuditsTable < ActiveRecord::Migration[8.1]
  def up
    drop_table :audits if table_exists?(:audits)
  end

  def down
    # No-op - we don't want to recreate the audits table
    # If needed, use paper_trail's versions table instead
  end
end
