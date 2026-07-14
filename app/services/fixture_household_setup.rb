# frozen_string_literal: true

module FixtureHouseholdSetup
  HOUSEHOLD_SLUG = 'fixture-household'
  OWNER_EMAILS = %w[
    admin@example.com
    damacus@example.com
    john.doe@example.com
  ].freeze
  TENANT_MODELS = [
    Person, Location, LocationMembership, Medication, MedicationDosageOption, Dosage, Schedule,
    PersonMedication, NotificationPreference
  ].freeze

  def self.apply!
    return unless ActiveRecord::Base.connection.table_exists?(:households)

    household = fixture_household
    assign_fixture_records(household)
    memberships = create_memberships(household)
    create_fixture_grants(household, memberships)
  end

  def self.fixture_household
    Household.find_or_create_by!(slug: HOUSEHOLD_SLUG) do |household|
      household.name = 'Fixture Household'
      household.timezone = Time.zone.name
    end
  end

  def self.assign_fixture_records(household)
    TENANT_MODELS.each do |model|
      next unless model.table_exists? && model.column_names.include?('household_id')

      model.where(household_id: nil).find_each do |record|
        record.household_id = household.id
        record.save!(validate: false)
      end
    end
    MedicationAdministration::HistoricalDataMigration.new.backfill_household(household: household)
  end

  def self.create_memberships(household)
    Person.includes(:account).filter_map do |person|
      next unless person.account

      membership = household.household_memberships.find_or_initialize_by(account: person.account)
      membership.person = person
      membership.role = OWNER_EMAILS.include?(person.account.email) ? :owner : :member
      membership.status = :active
      membership.save!
      [person, membership]
    end.to_h
  end

  def self.create_fixture_grants(household, memberships)
    owner_membership = memberships.values.find(&:owner?) || memberships.values.first
    memberships.each do |person, membership|
      grant_access(household: household, membership: membership, person: person, access_level: :manage,
                   relationship_type: :self, grantor: owner_membership)
      grant_fixture_relationships(household, person, membership, owner_membership)
    end
  end

  def self.grant_fixture_relationships(household, person, membership, owner_membership)
    grant_all_people(household, membership, :manage, owner_membership) if owner_fixture_person?(person)
    grant_all_people(household, membership, :view, owner_membership) if professional_fixture_person?(person)
    grant_patient_relationships(household, person, membership, owner_membership)
  end

  def self.grant_all_people(household, membership, access_level, owner_membership)
    household.people.find_each do |person|
      grant_access(household: household, membership: membership, person: person, access_level: access_level,
                   relationship_type: :family_member, grantor: owner_membership)
    end
  end

  def self.grant_patient_relationships(household, person, membership, owner_membership)
    person.active_patient_relationships.find_each do |relationship|
      grant_access(household: household, membership: membership, person: relationship.patient,
                   access_level: access_level_for_relationship(relationship),
                   relationship_type: grant_relationship_type_for(relationship), grantor: owner_membership,
                   carer_relationship: relationship)
    end
  end

  def self.access_level_for_relationship(relationship)
    return :record if relationship.relationship_type == 'professional_carer'

    :manage
  end

  def self.grant_relationship_type_for(relationship)
    return :self if relationship.relationship_type == '0'
    return :carer if relationship.relationship_type == 'professional_carer'

    relationship.relationship_type
  end

  def self.grant_access(attributes)
    household = attributes.fetch(:household)
    membership = attributes.fetch(:membership)
    person = attributes.fetch(:person)

    grant = household.person_access_grants.find_or_initialize_by(household_membership: membership, person: person)
    carer_relationship = attributes[:carer_relationship] if grant.new_record?
    grant.assign_attributes(
      access_level: attributes.fetch(:access_level),
      relationship_type: attributes.fetch(:relationship_type),
      granted_by_membership: attributes.fetch(:grantor),
      carer_relationship: carer_relationship || grant.carer_relationship
    )
    grant.save!
  end

  def self.owner_fixture_person?(person)
    OWNER_EMAILS.include?(person.account&.email)
  end

  def self.professional_fixture_person?(person)
    %w[doctor nurse].include?(person.professional_title.to_s)
  end
end
