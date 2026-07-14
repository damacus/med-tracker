# frozen_string_literal: true

module Households
  class LocalMigrator
    class Error < StandardError; end

    COUNT_TABLES = %w[accounts people locations medications dosages schedules person_medications medication_takes
                      notification_preferences households household_memberships person_access_grants].freeze

    TENANT_BACKFILL_TABLES = %w[people locations location_memberships medications dosages schedules
                                person_medications notification_preferences].freeze

    LEGACY_USER_ROLES = { 0 => :administrator, 1 => :doctor, 2 => :nurse, 3 => :carer, 4 => :parent,
                          5 => :minor }.freeze
    MANAGE_REL_TYPES = %w[self parent].freeze

    Result = Data.define(:applied, :household, :before_counts, :after_counts) do
      def applied? = applied

      def summary_lines
        [
          "mode=#{applied? ? 'apply' : 'dry-run'}",
          ("household_id=#{household.id}" if household),
          "before=#{before_counts.inspect}",
          "after=#{after_counts.inspect}"
        ].compact
      end
    end

    def initialize(owner_email:, household_name:, apply:)
      @owner_email = owner_email.to_s.strip.downcase
      @household_name = household_name.to_s.strip
      @apply = apply
    end

    def call
      validate_inputs!
      owner = owner_account
      before_counts = table_counts
      return dry_run_result(before_counts) unless apply

      household = nil
      ActiveRecord::Base.transaction do
        household = find_or_create_household(owner)
        backfill_tenant_rows(household)
        create_memberships(household, owner)
        create_self_grants(household)
        migrate_carer_relationship_grants(household)
      end

      Result.new(applied: true, household: household, before_counts: before_counts, after_counts: table_counts)
    end

    private

    attr_reader :owner_email, :household_name, :apply

    def dry_run_result(before_counts) = Result.new(false, nil, before_counts, before_counts)

    def validate_inputs!
      raise Error, 'OWNER_EMAIL is required' if owner_email.blank?
      raise Error, 'HOUSEHOLD_NAME is required' if household_name.blank?
    end

    def owner_account
      matches = Account.verified.where('LOWER(email) = ?', owner_email)
      return matches.sole if matches.one?

      raise Error, 'expected exactly one verified owner account matching OWNER_EMAIL'
    end

    def table_counts
      COUNT_TABLES.each_with_object({}) do |table_name, counts|
        next unless ActiveRecord::Base.connection.table_exists?(table_name)

        counts[table_name] = model_for_table(table_name).unscoped.count
      end
    end

    def find_or_create_household(owner)
      household = Household.find_or_initialize_by(name: household_name, created_by_account: owner)
      household.timezone ||= Time.zone.name
      household.subscription_plan = highest_subscription_plan
      household.save!
      household
    end

    def highest_subscription_plan
      return :free unless ActiveRecord::Base.connection.column_exists?(:accounts, :subscription_plan)

      Account.exists?(subscription_plan: 'family_plus') ? :family_plus : :free
    end

    def backfill_tenant_rows(household)
      deduplicate_locations_for_household

      TENANT_BACKFILL_TABLES.each do |table_name|
        next unless ActiveRecord::Base.connection.table_exists?(table_name)
        next unless ActiveRecord::Base.connection.column_exists?(table_name, :household_id)

        model_for_table(table_name).where(household_id: nil).find_each do |record|
          backfill_household(record, household)
        end
      end
      medication_administration_history_migration.backfill_household(household: household)

      household.locations.find_or_create_by!(name: 'Home')
    end

    def deduplicate_locations_for_household
      grouped_locations = Location.where(household_id: nil).order(:id).group_by do |location|
        location.name.downcase
      end

      grouped_locations.each_value do |locations|
        keeper, *duplicates = locations
        duplicates.each { |duplicate| merge_location!(from: duplicate, into: keeper) }
      end
    end

    def merge_location!(from:, into:)
      LocationMembership.where(location: from).find_each do |membership|
        if LocationMembership.exists?(person_id: membership.person_id, location_id: into.id)
          membership.destroy!
        else
          move_location_membership(membership, into)
        end
      end

      move_medications(from, into)
      medication_administration_history_migration.move_location(from: from, into: into)
      from.destroy!
    end

    def move_location_membership(membership, location) = write_column(membership, :location_id, location.id)

    def move_medications(from, into)
      Medication.where(location: from).find_each { |medication| write_column(medication, :location_id, into.id) }
    end

    def model_for_table(table_name)
      {
        'accounts' => Account,
        'dosages' => MedicationDosageOption
      }.fetch(table_name) { table_name.classify.constantize }
    end

    def backfill_household(record, household)
      write_column(record, :household_id, household.id)
    end

    def write_column(record, column_name, value)
      connection = record.class.connection
      table_name = connection.quote_table_name(record.class.table_name)
      column = connection.quote_column_name(column_name)
      sql = record.class.sanitize_sql_array(["UPDATE #{table_name} SET #{column} = ? WHERE id = ?", value, record.id])
      connection.execute(sql)
      record[column_name] = value
    end

    def medication_administration_history_migration
      @medication_administration_history_migration ||= MedicationAdministration::HistoricalDataMigration.new
    end

    def create_memberships(household, owner)
      LocalMembershipMigrator.new(
        household: household,
        owner: owner,
        accounts: Account.includes(person: :user),
        prepare_person: ->(person) { prepare_membership_person(person, household) },
        role_for: ->(account) { membership_role_for(account, owner) }
      ).call
    end

    def prepare_membership_person(person, household)
      return unless person

      backfill_household(person, household) if person
      migrate_professional_title(person)
    end

    def migrate_professional_title(person)
      legacy_role = legacy_user_role(person&.user)
      return unless %i[doctor nurse].include?(legacy_role)

      person.update!(professional_title: legacy_role)
    end

    def membership_role_for(account, owner)
      return :owner if account == owner
      return :administrator if legacy_user_role(account.person&.user) == :administrator

      :member
    end

    def legacy_user_role(user)
      return unless user&.has_attribute?(:role)

      raw_role = user.read_attribute_before_type_cast(:role)
      symbolic_role = raw_role.to_sym if raw_role.respond_to?(:to_sym)
      return symbolic_role if LEGACY_USER_ROLES.value?(symbolic_role)

      LEGACY_USER_ROLES[raw_role.to_i]
    end

    def create_self_grants(household)
      household.household_memberships.includes(:person).find_each do |membership|
        next if membership.person.blank?

        upsert_grant(
          membership: membership,
          person: membership.person,
          access_level: :manage,
          relationship_type: :self,
          granted_by_membership: membership
        )
      end
    end

    def migrate_carer_relationship_grants(household)
      CarerRelationship.active.includes(:carer, :patient).find_each do |relationship|
        membership = household.household_memberships.find_by(person: relationship.carer)
        next if membership.blank?

        upsert_grant(
          membership: membership,
          person: relationship.patient,
          access_level: access_level_for_relationship(relationship),
          relationship_type: relationship_type_for_grant(relationship),
          granted_by_membership: household.household_memberships.owner.active.first,
          carer_relationship: relationship
        )
      end
    end

    def access_level_for_relationship(rel) = MANAGE_REL_TYPES.include?(rel.relationship_type) ? :manage : :record

    def relationship_type_for_grant(relationship)
      return :professional if relationship.relationship_type == 'professional_carer'
      return relationship.relationship_type if PersonAccessGrant.relationship_types.key?(relationship.relationship_type)

      :family_member
    end

    def upsert_grant(membership:, person:, **attributes)
      actor_membership = attributes.fetch(:granted_by_membership, membership)
      GrantReconciler.new(
        membership: membership,
        person: person,
        attributes: attributes,
        access_change: AccessChange.for(actor_membership)
      ).call
    end

    class GrantReconciler
      def initialize(membership:, person:, attributes:, access_change:)
        @membership = membership
        @person = person
        @attributes = attributes
        @access_change = access_change
      end

      def call
        grant = current_grant
        return persist_new_grant!(grant, attributes) if grant.new_record?
        return preserve_manual_grant!(grant, attributes) if grant.carer_relationship_id.nil?
        return reconcile_relationship_grant!(grant, attributes) if same_relationship_source?(grant, attributes)

        raise LocalMigrator::Error, 'existing grant belongs to another relationship'
      end

      private

      attr_reader :membership, :person, :attributes, :access_change

      def current_grant
        PersonAccessGrant.find_or_initialize_by(
          household: membership.household,
          household_membership: membership,
          person: person,
          revoked_at: nil
        )
      end

      def persist_new_grant!(grant, attributes)
        access_change.upsert_grant!(
          grant,
          authority_attributes(attributes).merge(
            household: membership.household,
            household_membership: membership,
            person: person,
            granted_by_membership: attributes[:granted_by_membership],
            carer_relationship: attributes[:carer_relationship]
          )
        )
      end

      def preserve_manual_grant!(grant, attributes)
        return grant if grant.cover_access?(attributes[:access_level]) && grant.cover_expiry?(attributes[:expires_at])

        raise LocalMigrator::Error, 'existing manual grant does not cover the migrated relationship access'
      end

      def reconcile_relationship_grant!(grant, attributes)
        access_change.upsert_grant!(grant, authority_attributes(attributes))
      end

      def same_relationship_source?(grant, attributes)
        attributes[:carer_relationship] && grant.carer_relationship_id == attributes[:carer_relationship].id
      end

      def authority_attributes(attributes)
        {
          access_level: attributes[:access_level],
          relationship_type: attributes[:relationship_type],
          expires_at: attributes[:expires_at]
        }
      end
    end
  end
end
