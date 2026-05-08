# frozen_string_literal: true

class CreateExternalLookupAuditEvents < ActiveRecord::Migration[8.1]
  def change
    create_table :external_lookup_audit_events do |t|
      t.string :source, null: false
      t.string :event, null: false
      t.string :query_hash
      t.string :result_status, null: false
      t.integer :result_count, default: 0, null: false
      t.string :whodunnit
      t.string :ip
      t.string :request_id

      t.datetime :created_at, null: false, default: -> { 'CURRENT_TIMESTAMP' }
    end

    add_index :external_lookup_audit_events, :source
    add_index :external_lookup_audit_events, :created_at
    add_index :external_lookup_audit_events, :whodunnit
    add_index :external_lookup_audit_events, :result_status
  end
end
