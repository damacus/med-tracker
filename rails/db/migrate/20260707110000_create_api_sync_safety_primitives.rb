# frozen_string_literal: true

class CreateApiSyncSafetyPrimitives < ActiveRecord::Migration[8.1]
  def change
    create_table :api_idempotency_keys do |t|
      t.references :household, null: false, foreign_key: true
      t.references :account, null: false, foreign_key: true
      t.references :api_session, foreign_key: true
      t.references :api_app_token, foreign_key: true
      t.string :key, null: false
      t.string :request_method, null: false
      t.string :request_path, null: false
      t.string :request_digest, null: false
      t.integer :response_status, null: false
      t.jsonb :response_body, null: false, default: {}
      t.datetime :expires_at, null: false
      t.timestamps

      t.index %i[household_id key], unique: true
      t.index :expires_at
    end

    create_table :api_change_events do |t|
      t.references :household, null: false, foreign_key: true
      t.references :account, null: false, foreign_key: true
      t.references :household_membership, null: false, foreign_key: true
      t.string :request_id
      t.string :record_type, null: false
      t.bigint :record_id, null: false
      t.string :record_portable_id
      t.string :action, null: false
      t.jsonb :metadata, null: false, default: {}
      t.datetime :occurred_at, null: false
      t.timestamps

      t.index %i[household_id occurred_at]
      t.index %i[record_type record_id]
      t.index %i[household_id record_portable_id]
    end

    create_table :api_tombstones do |t|
      t.references :household, null: false, foreign_key: true
      t.references :account, null: false, foreign_key: true
      t.references :household_membership, null: false, foreign_key: true
      t.string :record_type, null: false
      t.string :record_portable_id, null: false
      t.string :action, null: false, default: 'delete'
      t.jsonb :metadata, null: false, default: {}
      t.datetime :deleted_at, null: false
      t.timestamps

      t.index %i[household_id deleted_at]
      t.index %i[household_id record_type record_portable_id], name: 'index_api_tombstones_on_household_record'
    end

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
