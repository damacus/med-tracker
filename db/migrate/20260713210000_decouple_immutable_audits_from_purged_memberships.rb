# frozen_string_literal: true

class DecoupleImmutableAuditsFromPurgedMemberships < ActiveRecord::Migration[8.1]
  def up
    remove_foreign_key :security_audit_events, column: :actor_membership_id
    remove_foreign_key :versions, column: :actor_membership_id
  end

  def down
    raise ActiveRecord::IrreversibleMigration, 'Purged memberships cannot be restored from immutable audit identifiers'
  end
end
