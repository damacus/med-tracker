# frozen_string_literal: true

class RemovePasskeysRailsTables < ActiveRecord::Migration[7.0]
  # rubocop:disable Rails/ReversibleMigration
  def change
    drop_table :passkeys_rails_passkeys if table_exists?(:passkeys_rails_passkeys)
    drop_table :passkeys_rails_agents if table_exists?(:passkeys_rails_agents)
  end
  # rubocop:enable Rails/ReversibleMigration
end
