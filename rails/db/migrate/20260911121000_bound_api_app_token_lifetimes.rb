class BoundApiAppTokenLifetimes < ActiveRecord::Migration[8.1]
  def up
    months = Integer(ENV.fetch('API_APP_TOKEN_MAX_AGE_MONTHS', '12'), 10)
    raise ArgumentError, 'API_APP_TOKEN_MAX_AGE_MONTHS must be positive' unless months.positive?

    add_column :api_app_tokens, :expires_at, :datetime
    execute <<~SQL
      UPDATE api_app_tokens
      SET expires_at = created_at + make_interval(months => #{months})
    SQL
    change_column_null :api_app_tokens, :expires_at, false
    add_index :api_app_tokens, :expires_at
    add_check_constraint :api_app_tokens, 'expires_at > created_at', name: 'api_app_token_positive_lifetime'
  end

  def down
    remove_check_constraint :api_app_tokens, name: 'api_app_token_positive_lifetime'
    remove_column :api_app_tokens, :expires_at
  end
end
