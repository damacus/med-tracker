# frozen_string_literal: true

# This table is managed internally by Rodauth's OmniAuth feature.
# It stores OAuth provider identities linked to accounts.
# No ActiveRecord model is needed as Rodauth handles all CRUD operations.
# See: https://github.com/janko/rodauth-omniauth
class CreateAccountIdentities < ActiveRecord::Migration[8.0]
  def change
    create_table :account_identities do |t|
      t.references :account, null: false, foreign_key: { on_delete: :cascade }
      t.string :provider, null: false
      t.string :uid, null: false
      t.timestamps

      t.index %i[provider uid], unique: true
    end
  end
end
