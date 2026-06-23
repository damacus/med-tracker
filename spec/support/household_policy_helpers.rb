# frozen_string_literal: true

module HouseholdPolicyHelpers
  def household_policy_member(role: :owner, household: nil, email: nil, name: nil)
    attributes = policy_member_attributes(role, household, email, name)
    return household_policy_owner(attributes) if attributes[:household].nil? && attributes[:role] == :owner

    household = attributes[:household] || default_policy_household
    account, person, membership = create_policy_member_record(household, attributes)
    policy_member_payload(account, household, membership, person)
  end

  def policy_member_attributes(role, household, email, name)
    role = role.to_sym
    { role: role, household: household, email: email || "policy-#{role}-#{SecureRandom.hex(6)}@example.test",
      name: name || "Policy #{role.to_s.titleize}" }
  end

  def household_policy_owner(attributes)
    account = Account.create!(email: attributes[:email], status: :verified)
    household = Household.create_with_owner!(name: "#{attributes[:name]} Household", owner_account: account,
                                             owner_person_attributes: person_attributes(attributes[:name]))
    membership = household.household_memberships.find_by!(account: account)
    policy_member_payload(account, household, membership, membership.person)
  end

  def create_policy_membership(household, account, person, role)
    household.household_memberships.create!(account: account, person: person, role: role, status: :active)
  end

  def default_policy_household = household_policy_member(role: :owner).fetch(:household)

  def create_policy_member_record(household, attributes)
    account = Account.create!(email: attributes[:email], status: :verified)
    person = household.people.create!(person_attributes(attributes[:name]).merge(account: account))
    [account, person, create_policy_membership(household, account, person, attributes[:role])]
  end

  def policy_member_payload(account, household, membership, person)
    { account: account, household: household, membership: membership, person: person,
      context: AuthorizationContext.new(account: account, household: household, membership: membership) }
  end

  def grant_policy_person_access(member, person, access_level: :view, relationship_type: :family_member)
    member.fetch(:household).person_access_grants.create!(
      household_membership: member.fetch(:membership),
      person: person,
      access_level: access_level,
      relationship_type: relationship_type,
      granted_by_membership: member.fetch(:membership)
    )
  end

  def household_policy_person(household, name: 'Policy Person', person_type: :adult, has_capacity: true)
    household.people.create!(
      name: "#{name} #{SecureRandom.hex(3)}",
      date_of_birth: 30.years.ago.to_date,
      person_type: person_type,
      has_capacity: has_capacity
    )
  rescue ActiveRecord::RecordInvalid
    person = household.people.new(
      name: "#{name} #{SecureRandom.hex(3)}",
      date_of_birth: 10.years.ago.to_date,
      person_type: person_type,
      has_capacity: has_capacity
    )
    person.save!(validate: false)
    person
  end

  def household_policy_location(household, name: nil)
    household.locations.create!(name: name || "Policy Location #{SecureRandom.hex(3)}")
  end

  def household_policy_medication(household, location: nil, name: nil)
    household.medications.create!(
      name: name || "Policy Medication #{SecureRandom.hex(3)}",
      location: location || household_policy_location(household),
      dosage_amount: 500,
      dosage_unit: 'mg',
      reorder_threshold: 0
    )
  end

  def household_policy_schedule(household, person:, medication: nil)
    household.schedules.create!(
      person: person,
      medication: medication || household_policy_medication(household),
      start_date: Time.zone.today,
      end_date: 1.month.from_now.to_date,
      dose_amount: 500,
      dose_unit: 'mg'
    )
  end

  def household_policy_person_medication(household, person:, medication: nil)
    household.person_medications.create!(
      person: person,
      medication: medication || household_policy_medication(household),
      dose_amount: 500,
      dose_unit: 'mg'
    )
  end

  def household_policy_medication_take(household, source:)
    attributes = {
      taken_at: Time.current,
      dose_amount: 500,
      dose_unit: 'mg'
    }
    attributes[source.is_a?(Schedule) ? :schedule : :person_medication] = source
    household.medication_takes.create!(attributes)
  end

  def person_attributes(name)
    { name: name, date_of_birth: 30.years.ago.to_date, person_type: :adult, has_capacity: true }
  end
end

RSpec.configure do |config|
  config.include HouseholdPolicyHelpers, type: :policy
end
