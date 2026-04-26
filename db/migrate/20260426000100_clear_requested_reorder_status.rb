# frozen_string_literal: true

class ClearRequestedReorderStatus < ActiveRecord::Migration[8.1]
  def up
    execute 'UPDATE medicines SET reorder_status = NULL WHERE reorder_status = 0'
  end

  def down
    raise ActiveRecord::IrreversibleMigration
  end
end
