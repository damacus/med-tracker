class CreateAuditLogs < ActiveRecord::Migration[8.1]
  def change
    create_table :audit_logs do |t|
      t.references :user, null: true, foreign_key: true
      t.string :action, null: false
      t.string :auditable_type, null: false
      t.integer :auditable_id, null: false
      t.text :changes
      t.string :ip_address
      t.string :user_agent
      t.timestamps

      t.index [:auditable_type, :auditable_id]
      t.index :action
      t.index :created_at
    end
  end
end
