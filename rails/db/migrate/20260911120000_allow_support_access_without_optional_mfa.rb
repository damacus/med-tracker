class AllowSupportAccessWithoutOptionalMfa < ActiveRecord::Migration[8.1]
  def change
    change_column_null :support_access_sessions, :mfa_verified_at, true
  end
end
