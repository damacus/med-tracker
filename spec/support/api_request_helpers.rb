# frozen_string_literal: true

module ApiRequestHelpers
  OWNER_FIXTURE_EMAILS = %w[
    admin@example.com
    damacus@example.com
    john.doe@example.com
  ].freeze

  module Auth
    def api_login(user, password: 'password', device_name: 'RSpec iPhone', household_id: nil)
      household_id ||= ensure_api_household_for(user).id
      login_params = {
        email: user.email_address,
        password: password,
        device_name: device_name
      }
      login_params[:household_id] = household_id if household_id

      post api_v1_auth_login_path,
           params: login_params,
           as: :json

      response.parsed_body.fetch('data')
    end

    def api_auth_headers(access_token)
      {
        'Authorization' => "Bearer #{access_token}",
        'Accept' => 'application/json'
      }
    end
  end

  module HouseholdSetup
    def ensure_api_household_for(user)
      membership = user.person.account.first_active_household_membership
      if membership&.active?
        if membership.person_id == user.person_id
          seed_household_records_for_api_user(user, membership.household, membership)
        end
        return membership.household
      end
      return ensure_membership_for_existing_person_household(user) if user.person.household

      create_api_household_for(user)
    end

    def ensure_membership_for_existing_person_household(user)
      household = user.person.household
      membership = membership_for_existing_person_household(user, household)
      ensure_household_owner!(household, excluding_membership: membership) unless membership_role_for(user) == :owner
      refresh_membership!(membership, user)
      seed_household_records_for_api_user(user, household, membership)
      household
    end

    def create_api_household_for(user)
      household = Household.create!(
        name: "API Helper #{SecureRandom.hex(4)}",
        slug: "api-helper-#{SecureRandom.hex(4)}"
      )
      user.person.update!(household: household)
      membership = create_api_membership(household, user)
      ensure_household_owner!(household, excluding_membership: membership) unless membership_role_for(user) == :owner
      seed_household_records_for_api_user(user, household, membership)
      household
    end

    def membership_for_existing_person_household(user, household)
      household.household_memberships.find_or_create_by!(account: user.person.account) do |household_membership|
        household_membership.person = user.person
        household_membership.role = membership_role_for(user)
        household_membership.status = :active
      end
    end

    def create_api_membership(household, user)
      household.household_memberships.create!(
        account: user.person.account,
        person: user.person,
        role: membership_role_for(user),
        status: :active
      )
    end

    def refresh_membership!(membership, user)
      return if membership_current?(membership, user)

      membership.update!(person: user.person, role: membership_role_for(user), status: :active)
    end

    def membership_current?(membership, user)
      membership.active? && membership.person_id == user.person_id && membership.role == membership_role_for(user).to_s
    end

    def membership_role_for(user)
      return :owner if owner_fixture_user?(user)

      :member
    end

    def owner_fixture_user?(user)
      OWNER_FIXTURE_EMAILS.include?(user.email_address)
    end

    def ensure_household_owner!(household, excluding_membership: nil)
      owner_scope = household.household_memberships.owner.active
      owner_scope = owner_scope.where.not(id: excluding_membership.id) if excluding_membership&.owner?
      return if owner_scope.exists?

      owner_account = Account.create!(
        email: "household-owner-#{SecureRandom.hex(8)}@example.test",
        status: :verified
      )
      household.household_memberships.create!(account: owner_account, role: :owner, status: :active)
    end
  end

  module HouseholdSeeding
    def seed_household_records_for_api_user(user, household, membership)
      people = seed_household_people(user, household)

      seed_household_memberships(people, household)
      grant_people_to_api_membership(user, people, household, membership)
      person_ids = people.map(&:id)
      original_location_ids = LocationMembership.where(person_id: person_ids).pluck(:location_id)
      seed_household_locations(user, household, person_ids)
      seed_household_medication_records(user, household, person_ids, original_location_ids)
      assign_household(NotificationPreference.where(person_id: person_ids), household)
    end

    def seed_household_people(user, household)
      people = assignable_household_people(user, household)
      assign_household(Person.where(id: people.map(&:id)), household)
      people.each { |person| person.household_id = household.id }
      people
    end

    def assignable_household_people(user, household)
      people = legacy_fixture_people_for(user)
      people.reject do |person|
        (person.household_id.present? && person.household_id != household.id) ||
          duplicate_household_account_person?(person, household)
      end
    end

    def duplicate_household_account_person?(person, household)
      person.account_id.present? &&
        household.people.where(account_id: person.account_id).where.not(id: person.id).exists?
    end

    def seed_household_memberships(people, household)
      people.each do |person|
        next unless person.account

        membership = household.household_memberships.find_or_initialize_by(account: person.account)
        membership.person ||= person
        membership.role ||= OWNER_FIXTURE_EMAILS.include?(person.account.email) ? :owner : :member
        membership.status = :active
        membership.save!
      end
    end

    def legacy_fixture_people_for(user)
      return Person.all.to_a if owner_fixture_user?(user) || professional_fixture_user?(user)

      people = [user.person]
      people.concat(user.person&.patients.to_a)
      people.compact.uniq
    end

    def grant_people_to_api_membership(user, people, household, membership)
      people.each do |person|
        grant = household.person_access_grants.find_or_initialize_by(household_membership: membership, person: person)
        grant.access_level = grant_access_level_for(user, person)
        grant.relationship_type = person.id == membership.person_id ? :self : :family_member
        grant.granted_by_membership = membership
        grant.save!
      end
    end

    def grant_access_level_for(user, person)
      return :manage if fixture_manage_access_for?(user, person)

      relationship = user.person&.active_patient_relationships&.find_by(patient: person)
      return access_level_for_relationship(relationship) if relationship

      return :view if professional_fixture_user?(user)

      :view
    end

    def fixture_manage_access_for?(user, person)
      owner_fixture_user?(user) ||
        self_care_relationship?(user, person) ||
        self_management_grant?(user, person)
    end

    def self_management_grant?(user, person)
      self_managing_adult?(user, person) && !professional_care_fixture_user?(user)
    end

    def inventory_manage_access_for?(user)
      !carer_fixture_user?(user) || self_managing_adult?(user, user.person)
    end

    def carer_fixture_user?(user)
      user.person&.patients&.any? || self_care_relationship?(user, user.person)
    end

    def access_level_for_relationship(relationship)
      return :manage if relationship.relationship_type == 'parent'

      :record
    end

    def professional_fixture_user?(user)
      %w[doctor nurse].include?(user.person&.professional_title.to_s)
    end

    def professional_care_fixture_user?(user)
      professional_fixture_user?(user) ||
        user.person&.active_patient_relationships&.exists?(relationship_type: 'professional_carer')
    end

    def self_managing_adult?(user, person)
      person.id == user.person_id && person.adult? && person.has_capacity?
    end

    def self_care_relationship?(user, person)
      person.id == user.person_id &&
        CarerRelationship.active.exists?(carer: person, patient: person, relationship_type: %w[self 0])
    end
  end

  module TenantRecordSeeding
    def seed_household_locations(user, household, person_ids)
      location_memberships = LocationMembership.where(person_id: person_ids)
      locations = locations_for_household_seed(user, location_memberships)

      seed_household_location_rows(locations, household, location_memberships, person_ids)
    end

    def locations_for_household_seed(user, location_memberships)
      return Location.all if owner_fixture_user?(user) || professional_fixture_user?(user)

      Location.where(id: location_memberships.select(:location_id))
    end

    def seed_household_location_rows(locations, household, location_memberships, person_ids)
      locations.find_each do |location|
        target = household_location_for(location, household, person_ids)
        LocationMembership.where(id: location_memberships.where(location_id: location.id).select(:id))
                          .find_each { |membership| membership.update!(location: target, household: household) }
      end
    end

    def household_location_for(location, household, person_ids)
      return location if location.household_id == household.id

      existing = matching_household_location(location, household)
      return existing if existing

      unless shared_location_outside_people?(location, person_ids)
        return assign_location_to_household(location, household)
      end

      household.locations.create!(name: location.name, description: location.description)
    end

    def matching_household_location(location, household)
      household.locations.where('LOWER(name) = ?', location.name.downcase).first
    end

    def shared_location_outside_people?(location, person_ids)
      location.medications.exists? ||
        LocationMembership.where(location_id: location.id).where.not(person_id: person_ids).exists?
    end

    def assign_location_to_household(location, household)
      assign_household(Location.where(id: location.id), household)
      location.reload
    end

    def seed_medication_locations(household, medications)
      medications.includes(:location).find_each do |medication|
        next unless medication.location
        next if medication.location.household_id == household.id

        target = matching_household_location(medication.location, household) ||
                 household.locations.create!(
                   name: medication.location.name,
                   description: medication.location.description
                 )
        medication.update!(location: target)
      end
    end

    def seed_household_medication_records(user, household, person_ids, original_location_ids)
      schedules = Schedule.where(person_id: person_ids)
      person_medications = PersonMedication.where(person_id: person_ids)
      medication_ids = medication_ids_for_api_seed(
        user, household, schedules, person_medications, original_location_ids
      )

      seed_household_medication_parents(household, medication_ids)
      assign_household(schedules, household)
      assign_household(person_medications, household)
      seed_household_medication_takes(household, schedules, person_medications)
    end

    def medication_ids_for_api_seed(user, household, schedules, person_medications, original_location_ids)
      ids = schedules.pluck(:medication_id) + person_medications.pluck(:medication_id)
      ids.concat(person_location_medication_ids(original_location_ids)) if inventory_manage_access_for?(user)
      ids.concat(household_location_medication_ids(household)) if inventory_manage_access_for?(user)
      ids.compact.uniq
    end

    def person_location_medication_ids(location_ids)
      Medication.where(location_id: location_ids).pluck(:id)
    end

    def household_location_medication_ids(household)
      Medication.where(location_id: Location.where(household: household).select(:id)).pluck(:id)
    end

    def seed_household_medication_parents(household, medication_ids)
      medications = Medication.where(id: medication_ids)
      seed_medication_locations(household, medications)
      assign_household(medications, household)
      assign_household(MedicationDosageOption.where(medication_id: medication_ids), household)
    end

    def seed_household_medication_takes(household, schedules, person_medications)
      takes = MedicationTake.where(schedule_id: schedules.select(:id)).or(
        MedicationTake.where(person_medication_id: person_medications.select(:id))
      )

      assign_household(takes, household)
    end

    def assign_household(scope, household)
      connection = scope.klass.connection
      table_name = scope.klass.quoted_table_name

      scope.pluck(:id).each do |id|
        connection.execute(<<~SQL.squish)
          UPDATE #{table_name}
          SET household_id = #{connection.quote(household.id)}
          WHERE id = #{connection.quote(id)}
        SQL
      end
    end
  end

  include Auth
  include HouseholdSetup
  include HouseholdSeeding
  include TenantRecordSeeding
end

RSpec.configure do |config|
  config.include ApiRequestHelpers, type: :request
  config.include ApiRequestHelpers, type: :system
  config.include ApiRequestHelpers, type: :feature
end
