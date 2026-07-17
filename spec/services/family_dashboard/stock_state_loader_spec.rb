# frozen_string_literal: true

require 'rails_helper'

RSpec.describe FamilyDashboard::StockStateLoader do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages, :schedules,
           :person_medications, :medication_takes, :carer_relationships

  before { FixtureHouseholdSetup.apply! }
  after { Current.reset }

  it 'does not mark a source recordable without a person mutation grant' do
    selected_user = users(:doctor)
    household = selected_user.person.household
    membership = household.household_memberships.find_by!(account: selected_user.person.account)
    person = people(:john)
    grant = PersonAccessGrant.active.find_by!(household_membership: membership, person: person)
    grant.update!(revoked_at: Time.current)
    sources = administration_sources(household, person)
    loader = load_states(sources, selected_user, household, membership)

    expect(sources.map { |source| loader.state_for(source).fetch(:can_record) }).to eq([false, false])
  end

  it 'does not let an active support session bypass a view-only person grant' do
    selected_user = users(:admin)
    household = selected_user.person.household
    membership = household.household_memberships.find_by!(account: selected_user.person.account)
    membership.update!(role: :member)
    PersonAccessGrant.active.find_by!(household_membership: membership, person: selected_user.person).view!
    start_support_session(selected_user, household)
    sources = administration_sources(household, selected_user.person)
    loader = load_states(sources, selected_user, household, membership)

    expect(sources.map { |source| loader.state_for(source).fetch(:can_record) }).to eq([false, false])
  end

  def authorization_context(user, household, membership)
    AuthorizationContext.new(account: user.person.account, household: household, membership: membership)
  end

  def administration_sources(household, person)
    [
      create_source(household:, person:),
      create_source(household:, person:, source_class: PersonMedication)
    ]
  end

  def load_states(sources, user, household, membership)
    described_class.new(
      sources: sources,
      person_ids: sources.map(&:person_id).uniq,
      current_user: authorization_context(user, household, membership),
      date: Date.current,
      now: Time.current
    ).call
  end

  def start_support_session(user, household)
    platform_admin = PlatformAdmin.create!(account: user.person.account)
    Current.support_access_session = SupportAccessSession.create!(
      platform_admin: platform_admin,
      household: household,
      reason: 'Dashboard policy parity',
      mfa_verified_at: Time.current,
      starts_at: 1.minute.ago,
      expires_at: 30.minutes.from_now
    )
  end

  def create_source(household:, person:, source_class: Schedule)
    medication = create(:medication, household: household)
    attributes = {
      household: household,
      person: person,
      medication: medication,
      dosage: nil,
      dose_amount: medication.dose_amount,
      dose_unit: medication.dose_unit,
      min_hours_between_doses: nil
    }
    attributes[:schedule_type] = :prn if source_class == Schedule
    attributes[:frequency] = 'As needed' if source_class == Schedule
    attributes[:administration_kind] = :as_needed if source_class == PersonMedication

    create(source_class.model_name.singular.to_sym, **attributes)
  end
end
