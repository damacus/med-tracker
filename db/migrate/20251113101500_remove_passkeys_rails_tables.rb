# frozen_string_literal: true

class RemovePasskeysRailsTables < ActiveRecord::Migration[7.0]
  def change
    drop_table :passkeys_rails_passkeys
    drop_table :passkeys_rails_agents
  end
end
