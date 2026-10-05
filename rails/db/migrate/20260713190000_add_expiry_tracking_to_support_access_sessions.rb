# frozen_string_literal: true

class AddExpiryTrackingToSupportAccessSessions < ActiveRecord::Migration[8.1]
  def change
    add_column :support_access_sessions, :expired_at, :datetime
    add_index :support_access_sessions, %i[ended_at expired_at expires_at],
              name: 'index_support_access_sessions_for_expiry_processing'
  end
end
