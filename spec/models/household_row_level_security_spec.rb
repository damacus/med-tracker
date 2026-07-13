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

  def household_person(household_name:, slug:, person_name:)
    described_class.create!(name: household_name, slug: slug).people.create!(
      name: person_name,
      date_of_birth: 30.years.ago.to_date,
      person_type: :adult,
      has_capacity: true
    )
  end

  def carer_relationship_for(household, prefix:)
    CarerRelationship.create!(
      carer: household.people.create!(name: "#{prefix} Carer", date_of_birth: 30.years.ago.to_date),
      patient: household.people.create!(name: "#{prefix} Patient", date_of_birth: 10.years.ago.to_date),
      relationship_type: 'parent'
    )
  end

  def rls_household(label)
    described_class.create!(name: "RLS #{label} Household", slug: "rls-#{label.parameterize}")
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

  it 'keeps the runtime role separate from owner-capable migration privileges' do
    runtime_inherits_owner = connection.select_value(<<~SQL.squish)
      SELECT pg_has_role('med_tracker_app', 'med_tracker_owner', 'member')
    SQL
    runtime_can_create_public_objects = connection.select_value(<<~SQL.squish)
      SELECT has_schema_privilege('med_tracker_app', 'public', 'CREATE')
    SQL

    expect(runtime_inherits_owner).to be(false)
    expect(runtime_can_create_public_objects).to be(false)
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

  it 'does not expose null-tenant rows through household RLS policies' do
    null_visible_policies = connection.select_values(<<~SQL.squish)
      SELECT tablename
      FROM pg_policies
      WHERE schemaname = 'public'
        AND (
          lower(COALESCE(qual, '')) LIKE '%household_id is null%'
          OR lower(COALESCE(with_check, '')) LIKE '%household_id is null%'
        )
      ORDER BY tablename
    SQL

    expect(null_visible_policies).to be_empty
  end

  it 'default-denies household rows for the runtime role without tenant context' do
    household = described_class.create!(name: 'RLS Missing Context', slug: 'rls-missing-context')
    location = household.locations.create!(name: 'RLS Home')

    with_runtime_role do
      expect(Location.where(id: location.id).count).to eq(0)
    end
  end

  it 'allows account-linked person lookup before household context is resolved' do
    account, _membership = household_membership_for(
      email: 'rls-login-person@example.test',
      household_name: 'RLS Login Person Household',
      person_name: 'RLS Login Person'
    )
    other_person = household_person(
      household_name: 'RLS Other Person Household',
      slug: 'rls-other-person-household',
      person_name: 'RLS Other Person'
    )

    with_runtime_role do
      expect(Person.where(id: [account.person.id, other_person.id]).pluck(:id))
        .to contain_exactly(account.person.id)
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

  it 'isolates carer relationships by current household' do
    household = described_class.create!(name: 'RLS Relationship Household', slug: 'rls-relationship-household')
    other_household = described_class.create!(name: 'RLS Hidden Relationship Household',
                                              slug: 'rls-hidden-relationship-household')
    visible_relationship = carer_relationship_for(household, prefix: 'Visible')
    hidden_relationship = carer_relationship_for(other_household, prefix: 'Hidden')

    with_runtime_role(household: household) do
      expect(CarerRelationship.where(id: [visible_relationship.id, hidden_relationship.id]).pluck(:id))
        .to contain_exactly(visible_relationship.id)
    end
  end

  it 'rejects inserting a relationship for another household through the runtime role' do
    household = rls_household('Insert')
    other_household = rls_household('Foreign Insert')
    foreign_relationship = carer_relationship_for(other_household, prefix: 'Foreign Insert')

    with_runtime_role(household: household) do
      expect do
        connection.execute(<<~SQL.squish)
          INSERT INTO carer_relationships
            (household_id, carer_id, patient_id, relationship_type, active, created_at, updated_at)
          VALUES
            (#{foreign_relationship.household_id}, #{foreign_relationship.carer_id},
             #{foreign_relationship.patient_id}, 'parent', TRUE, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
        SQL
      end.to raise_error(ActiveRecord::StatementInvalid, /row-level security policy/)
    end
  end

  it 'rejects moving a visible relationship to another household through the runtime role' do
    household = rls_household('Update')
    other_household = rls_household('Foreign Update')
    visible_relationship = carer_relationship_for(household, prefix: 'Visible Update')
    foreign_relationship = carer_relationship_for(other_household, prefix: 'Foreign Update')

    with_runtime_role(household: household) do
      expect do
        connection.execute(<<~SQL.squish)
          UPDATE carer_relationships
          SET household_id = #{foreign_relationship.household_id},
              carer_id = #{foreign_relationship.carer_id},
              patient_id = #{foreign_relationship.patient_id}
          WHERE id = #{visible_relationship.id}
        SQL
      end.to raise_error(ActiveRecord::StatementInvalid, /row-level security policy/)
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
