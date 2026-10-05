# frozen_string_literal: true

module Admin
  class UserProvisioner
    Result = Data.define(:success?, :user, :error)

    def initialize(user:, password:, household:, actor_membership:)
      @user = user
      @password = password
      @household = household
      @actor_membership = actor_membership
    end

    def call
      error = validation_error
      return Result.new(false, user, error) if error

      persist_user!
      Result.new(true, user, nil)
    rescue ActiveRecord::RecordInvalid => e
      copy_account_email_errors(e.record)
      Result.new(false, user, :invalid_record)
    rescue CareDelegation::Assign::InvalidAccessLevel
      user.errors.add(:dependent_access_level, :inclusion)
      Result.new(false, user, :invalid_access_level)
    end

    private

    attr_reader :user, :password, :household, :actor_membership

    def validation_error
      return :invalid_membership_role unless membership_role_valid?

      :duplicate_account if account_already_exists?
    end

    def membership_role_valid?
      return true if MembershipRoleUpdater::ALLOWED_ROLES.include?(membership_role)

      user.errors.add(:membership_role, membership_role_error)
      false
    end

    def membership_role_error
      key = if membership_role == MembershipRoleUpdater::OWNER_ROLE
              'admin.membership_roles.owner_rejected'
            else
              'admin.membership_roles.invalid_role'
            end
      I18n.t(key)
    end

    def account_already_exists?
      return false unless Account.exists?(email: user.email_address)

      user.errors.add(:email_address, :taken)
      true
    end

    def persist_user!
      ActiveRecord::Base.transaction do
        account = create_account!
        save_user!(account)
        membership = create_membership!(account)
        grant_person_access!(membership, user.person, :manage, :self)
        assign_dependents!
      end
    end

    def create_account!
      Account.create!(
        email: user.email_address,
        password_hash: BCrypt::Password.create(password),
        status: :verified
      )
    end

    def save_user!(account)
      user.person.account = account
      user.person.household ||= household
      user.save!
    end

    def create_membership!(account)
      return unless household

      Households::AccessChange.for(actor_membership).create_membership!(
        household: household,
        account: account,
        person: user.person,
        role: membership_role,
        status: :active
      )
    end

    def assign_dependents!
      return if carer_relationship_type.blank?

      DependentRelationshipAssigner.new(
        carer: user.person,
        dependent_ids: user.dependent_ids,
        relationship_type: carer_relationship_type,
        access_level: user.dependent_access_level,
        scope: dependent_assignment_scope,
        granted_by_membership: actor_membership
      ).call
    end

    def dependent_assignment_scope
      return Person.none unless household

      Person.where(household: household)
    end

    def grant_person_access!(membership, person, access_level, relationship_type)
      return unless household && membership && person&.household_id == household.id

      grant = household.person_access_grants.find_or_initialize_by(
        household_membership: membership,
        person: person
      )
      attributes = {
        household: household,
        household_membership: membership,
        person: person,
        access_level: access_level,
        relationship_type: relationship_type,
        granted_by_membership: grant.granted_by_membership || actor_membership || membership,
        revoked_at: nil
      }
      access_change(membership).upsert_grant!(grant, attributes)
    end

    def access_change(membership)
      grantor = actor_membership || membership
      Households::AccessChange.for(grantor)
    end

    def membership_role
      requested_role = user.membership_role.presence
      return requested_role if MembershipRoleUpdater::ALLOWED_ROLES.include?(requested_role)

      requested_role || 'member'
    end

    def dependent_relationship_type
      requested_type = user.dependent_relationship_type.presence
      return requested_type if PersonAccessGrant.relationship_types.key?(requested_type) && requested_type != 'self'

      :professional
    end

    def carer_relationship_type
      case dependent_relationship_type.to_s
      when 'parent'
        'parent'
      when 'family_member'
        'family_member'
      else
        'professional_carer'
      end
    end

    def copy_account_email_errors(record)
      return unless record.is_a?(Account) && record.errors[:email].any?

      record.errors[:email].each do |error|
        user.errors.add(:email_address, error)
      end
    end
  end
end
