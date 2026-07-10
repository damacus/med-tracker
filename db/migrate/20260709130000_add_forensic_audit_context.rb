# frozen_string_literal: true

class AddForensicAuditContext < ActiveRecord::Migration[8.1]
  def change
    add_column :versions, :object_changes, :text
    add_column :versions, :audit_context, :jsonb, default: {}, null: false
    add_column :security_audit_events, :audit_context, :jsonb, default: {}, null: false
  end
end
