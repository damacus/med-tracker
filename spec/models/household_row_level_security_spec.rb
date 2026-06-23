# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Household do
  delegate :connection, to: :'ActiveRecord::Base'

  def with_runtime_role(account: nil, household: nil)
    connection.transaction(requires_new: true) do
      connection.execute('SET LOCAL ROLE med_tracker_app')
      connection.execute("SELECT set_config('med_tracker.current_account_id', '#{account.id}', true)") if account
      connection.execute("SELECT set_config('med_tracker.current_household_id', '#{household.id}', true)") if household
      yield
      raise ActiveRecord::Rollback
    end
  end

  def household_membership_for(email:, household_name:, person_name:)
    account = Account.create!(email: email, status: :verified)
    household = described_class.create_with_owner!(
      name: household_name,
      owner_account: account,
      owner_person_attributes: {
        name: person_name,
        date_of_birth: 30.years.ago.to_date,
        person_type: :adult,
        has_capacity: true
      }
    )

    [account, household.household_memberships.sole]
  end

  def household_person_with_avatar(household, name:, filename:)
    person = household.people.create!(
      name: name,
      date_of_birth: 30.years.ago.to_date,
      person_type: :adult,
      has_capacity: true
    )
    person.avatar.attach(io: StringIO.new(filename), filename: filename, content_type: 'image/png')
    person
  end

  it 'configures the runtime role without superuser or bypassrls' do
    role = connection.select_one(<<~SQL.squish)
      SELECT rolsuper, rolbypassrls
      FROM pg_roles
      WHERE rolname = 'med_tracker_app'
    SQL

    expect(role).to include('rolsuper' => false, 'rolbypassrls' => false)
  end

  it 'uses a separate owner role for application objects and grants runtime access' do
    people_owner = connection.select_value(<<~SQL.squish)
      SELECT pg_get_userbyid(relowner)
      FROM pg_class
      WHERE oid = 'people'::regclass
    SQL
    runtime_can_select_people = connection.select_value(<<~SQL.squish)
      SELECT has_table_privilege('med_tracker_app', 'people', 'SELECT')
    SQL
    login_has_owner_role = connection.select_value(<<~SQL.squish)
      SELECT pg_has_role(session_user, 'med_tracker_owner', 'member')
    SQL

    expect(people_owner).to eq('med_tracker_owner')
    expect(runtime_can_select_people).to be(true)
    expect(login_has_owner_role).to be(true)
  end

  it 'forces row-level security on every household-owned table' do
    missing = SchemaInventory.household_owned_tables.filter_map do |table_name|
      next unless connection.table_exists?(table_name)

      row = connection.select_one(<<~SQL.squish)
        SELECT relrowsecurity, relforcerowsecurity
        FROM pg_class
        WHERE oid = #{connection.quote(table_name)}::regclass
      SQL
      table_name unless row == { 'relrowsecurity' => true, 'relforcerowsecurity' => true }
    end

    expect(missing).to be_empty
  end

  it 'default-denies household rows for the runtime role without tenant context' do
    household = described_class.create!(name: 'RLS Missing Context', slug: 'rls-missing-context')
    location = household.locations.create!(name: 'RLS Home')

    with_runtime_role do
      expect(Location.where(id: location.id).count).to eq(0)
    end
  end

  it 'allows the runtime role to see only the current household rows' do
    household = described_class.create!(name: 'RLS Visible Household', slug: 'rls-visible-household')
    other_household = described_class.create!(name: 'RLS Hidden Household', slug: 'rls-hidden-household')
    visible_location = household.locations.create!(name: 'RLS Home')
    hidden_location = other_household.locations.create!(name: 'RLS Home')

    with_runtime_role(household: household) do
      expect(Location.where(id: [visible_location.id,
                                 hidden_location.id]).pluck(:id)).to contain_exactly(visible_location.id)
    end
  end

  it 'allows account-scoped household membership lookup before household context is resolved' do
    account, membership = household_membership_for(
      email: 'rls-member@example.test',
      household_name: 'RLS Member Household',
      person_name: 'RLS Member'
    )
    _, other_membership = household_membership_for(
      email: 'rls-other-member@example.test',
      household_name: 'RLS Other Member Household',
      person_name: 'RLS Other Member'
    )

    with_runtime_role(account: account) do
      expect(HouseholdMembership.where(id: [membership.id, other_membership.id]).pluck(:id))
        .to contain_exactly(membership.id)
    end
  end

  it 'isolates active storage attachments by current household' do
    household = described_class.create!(name: 'RLS Attachment Household', slug: 'rls-attachment-household')
    other_household = described_class.create!(name: 'RLS Other Attachment Household',
                                              slug: 'rls-other-attachment-household')
    visible_person = household_person_with_avatar(household, name: 'RLS Attachment Person', filename: 'visible.png')
    hidden_person = household_person_with_avatar(other_household, name: 'RLS Hidden Attachment Person',
                                                                  filename: 'hidden.png')

    with_runtime_role(household: household) do
      expect(ActiveStorage::Attachment.where(id: [visible_person.avatar.attachment.id,
                                                  hidden_person.avatar.attachment.id]).pluck(:id))
        .to contain_exactly(visible_person.avatar.attachment.id)
    end
  end
end
