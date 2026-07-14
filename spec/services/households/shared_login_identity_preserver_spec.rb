# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Households::SharedLoginIdentityPreserver do
  let(:target_household) { create(:household) }
  let(:surviving_household) { create(:household) }
  let(:operator) { Account.create!(email: 'identity-preserver-operator@example.test', status: :verified) }

  before { PlatformAdmin.create!(account: operator) }

  it 'preserves the same User through a clean surviving identity under forced RLS', :aggregate_failures do
    records = shared_login_records

    with_runtime_role do
      described_class.call(household: target_household, actor_account: operator)

      expect(preserved_identity_state(records)).to eq(
        identity_replaced: true,
        identity_household: surviving_household,
        identity_account: records.fetch(:account),
        identity_location_count: 0,
        membership_linked: true,
        old_identity_visible_then_deleted: true,
        account_resolves_user: true
      )
    end
  end

  it 'removes a target User when its account has no other operational household' do
    account = Account.create!(email: 'identity-preserver-single@example.test', status: :verified)
    person = create(:person, household: target_household, account: account)
    user = User.create!(person: person, email_address: account.email, password: 'password')

    described_class.call(household: target_household, actor_account: operator)

    expect(User.where(id: user.id)).not_to exist
  end

  def shared_login_records
    account = Account.create!(email: 'identity-preserver-shared@example.test', status: :verified)
    person = target_identity_person(account)
    user = User.create!(person: person, email_address: account.email, password: 'password')
    membership = surviving_household.household_memberships.create!(
      account: account,
      role: :owner,
      status: :active
    )
    { account: account, person: person, user: user, membership: membership }
  end

  def target_identity_person(account)
    Person.new(
      household: target_household,
      account: account,
      name: 'Shared target identity',
      date_of_birth: 30.years.ago.to_date,
      person_type: :adult,
      has_capacity: true
    ).tap { |person| person.save!(validate: false) }
  end

  def with_runtime_role
    ActiveRecord::Base.connection.transaction(requires_new: true) do
      ActiveRecord::Base.connection.execute('SET LOCAL ROLE med_tracker_app')
      yield
      raise ActiveRecord::Rollback
    end
  end

  def preserved_identity_state(records)
    select_runtime_household(surviving_household)
    identity = records.fetch(:user).reload.person
    state = surviving_identity_state(records, identity)
    deletion_state = remove_target_identity(records)
    select_runtime_household(surviving_household)

    state.merge(
      deletion_state,
      account_resolves_user: records.fetch(:account).reload.person.user == records.fetch(:user)
    )
  end

  def surviving_identity_state(records, identity)
    {
      identity_replaced: identity != records.fetch(:person),
      identity_household: identity.household,
      identity_account: identity.account,
      identity_location_count: identity.location_memberships.size,
      membership_linked: records.fetch(:membership).reload.person == identity
    }
  end

  def remove_target_identity(records)
    select_runtime_household(target_household)
    target = Person.where(id: records.fetch(:person).id)
    visible = target.exists?
    target.delete_all
    { old_identity_visible_then_deleted: visible && !target.exists? }
  end

  def select_runtime_household(household)
    ActiveRecord::Base.connection.execute(
      "SELECT set_config('med_tracker.current_household_id', '#{household.id}', true)"
    )
  end
end
