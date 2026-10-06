# frozen_string_literal: true

class RetireApiOidcNonces < ActiveRecord::Migration[8.1]
  def up
    execute 'LOCK TABLE api_oidc_nonces IN ACCESS EXCLUSIVE MODE'
    row_count = select_value('SELECT COUNT(*) FROM api_oidc_nonces').to_i
    raise ActiveRecord::MigrationError, "api_oidc_nonces contains #{row_count} row(s); refusing to drop it" if row_count.positive?

    drop_table :api_oidc_nonces
  end

  def down
    create_table :api_oidc_nonces do |t|
      t.string :issuer, null: false
      t.string :subject, null: false
      t.string :nonce, null: false
      t.datetime :used_at, null: false
      t.timestamps

      t.index %i[issuer subject nonce], unique: true
      t.index :used_at
    end
  end
end
