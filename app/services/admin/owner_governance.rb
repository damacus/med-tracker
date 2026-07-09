# frozen_string_literal: true

module Admin
  class OwnerGovernance
    def initialize(household:, actor_membership:)
      @household = household
      @actor_membership = actor_membership
    end

    def can_change_owner_membership?(target_membership)
      !owner_membership?(target_membership) || actor_membership&.owner?
    end

    def can_deactivate_owner_user?(target_membership)
      return true unless owner_membership?(target_membership)

      actor_membership&.owner? && usable_owner_count_excluding(target_membership).positive?
    end

    private

    attr_reader :household, :actor_membership

    def owner_membership?(membership)
      membership&.owner? && membership&.active?
    end

    def usable_owner_count_excluding(target_membership)
      return 0 unless household && target_membership

      household.household_memberships.owner.active
               .where.not(id: target_membership.id)
               .joins(account: { person: :user })
               .where(users: { active: true })
               .count
    end
  end
end
