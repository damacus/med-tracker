# frozen_string_literal: true

class CreateHouseholdTenancyFoundation < ActiveRecord::Migration[8.1]
  TENANT_TABLES = %i[
    people
    locations
    location_memberships
    medications
    medication_dosage_options
    dosages
    schedules
    person_medications
    medication_takes
    notification_preferences
  ].freeze

  def change
    create_table :households do |t|
      t.string :name, null: false
      t.string :slug, null: false
      t.string :status, null: false, default: 'active'
      t.string :timezone, null: false
      t.string :subscription_plan, null: false, default: 'free'
      t.references :created_by_account, foreign_key: { to_table: :accounts }

      t.timestamps
    end

    add_index :households, :slug, unique: true

    add_tenant_columns

    create_table :household_memberships do |t|
      t.references :household, null: false, foreign_key: true
      t.references :account, null: false, foreign_key: true
      t.references :person, foreign_key: true, index: false
      t.string :role, null: false, default: 'member'
      t.string :status, null: false, default: 'active'
      t.integer :permissions_version, null: false, default: 1
      t.datetime :joined_at, null: false
      t.datetime :revoked_at

      t.timestamps
    end

    add_index :household_memberships, %i[household_id account_id], unique: true
    add_index :household_memberships, :person_id, unique: true, where: 'person_id IS NOT NULL'

    create_table :person_access_grants do |t|
      t.references :household, null: false, foreign_key: true
      t.references :household_membership, null: false, foreign_key: true
      t.references :person, null: false, foreign_key: true
      t.string :access_level, null: false
      t.string :relationship_type, null: false
      t.references :granted_by_membership, foreign_key: { to_table: :household_memberships }
      t.datetime :expires_at
      t.datetime :revoked_at

      t.timestamps
    end

    add_index :person_access_grants,
              %i[household_membership_id person_id],
              unique: true,
              where: 'revoked_at IS NULL'

    create_table :household_invitations do |t|
      t.references :household, null: false, foreign_key: true
      t.references :invited_by_membership, null: false, foreign_key: { to_table: :household_memberships }
      t.citext :email, null: false
      t.string :membership_role, null: false
      t.string :token_digest, null: false
      t.datetime :expires_at, null: false
      t.datetime :accepted_at
      t.datetime :revoked_at

      t.timestamps
    end

    add_index :household_invitations, :token_digest, unique: true
    add_index :household_invitations,
              %i[household_id email],
              unique: true,
              where: 'accepted_at IS NULL AND revoked_at IS NULL'

    create_table :household_invitation_grants do |t|
      t.references :household_invitation, null: false, foreign_key: true
      t.references :person, null: false, foreign_key: true
      t.string :access_level, null: false
      t.string :relationship_type, null: false
      t.datetime :expires_at

      t.timestamps
    end

    create_table :security_audit_events do |t|
      t.references :household, foreign_key: true
      t.references :actor_account, foreign_key: { to_table: :accounts }
      t.references :actor_membership, foreign_key: { to_table: :household_memberships }
      t.string :event_type, null: false
      t.jsonb :metadata, null: false, default: {}
      t.string :request_id
      t.string :ip

      t.timestamps
    end

    add_index :security_audit_events, %i[household_id created_at]
    add_index :security_audit_events, :event_type

    add_index :people,
              %i[household_id account_id],
              unique: true,
              where: 'household_id IS NOT NULL AND account_id IS NOT NULL'
    remove_index :locations, name: 'index_locations_on_name', if_exists: true
    add_index :locations,
              'household_id, lower(name)',
              unique: true,
              where: 'household_id IS NOT NULL',
              name: 'index_locations_on_household_id_and_lower_name'
  end

  private

  def add_tenant_columns
    TENANT_TABLES.each do |table_name|
      next unless table_exists?(table_name)

      add_reference table_name, :household, foreign_key: true unless column_exists?(table_name, :household_id)
      add_index table_name, %i[id household_id], unique: true
    end

    add_column :people, :professional_title, :string
  end
end
