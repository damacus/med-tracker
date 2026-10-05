# frozen_string_literal: true

module Households
  class SharedLoginIdentityPreserver
    class << self
      def call(household:, actor_account:)
        target_people(household, actor_account).each do |person|
          next if person.user.blank?

          preserve_or_remove_identity!(person, household, actor_account)
        end
      end

      private

      def target_people(household, actor_account)
        TenantContext.with(account: actor_account, household: household) do
          Person.where(household: household).includes(:account, :user).to_a
        end
      end

      def preserve_or_remove_identity!(person, household, actor_account)
        membership = surviving_membership(person.account, household)
        return User.where(id: person.user.id).delete_all if membership.blank?

        preserve_identity!(person, membership, actor_account)
      end

      def surviving_membership(account, household)
        return if account.blank?

        TenantContext.with(account: account, household: household) do
          HouseholdMembership.active.joins(:household).merge(Household.operational)
                             .where(account: account).where.not(household: household)
                             .includes(:household, :person).order(:id).first
        end
      end

      def preserve_identity!(person, membership, actor_account)
        TenantContext.with(account: person.account, household: membership.household, membership: membership) do
          identity = reusable_identity(person.account, membership.household) ||
                     build_identity(person, membership.household)
          identity.save!(validate: false) if identity.new_record?
          person.user.update!(person: identity)
          update_membership_identity!(membership, identity, actor_account) if membership.person.blank?
        end
      end

      def update_membership_identity!(membership, identity, actor_account)
        AccessChange.new(actor_account: actor_account, actor_membership: nil, request: nil)
                    .update_membership!(membership, person: identity)
      end

      def reusable_identity(account, household)
        Person.where(household: household, account: account).left_outer_joins(:user).find_by(users: { id: nil })
      end

      def build_identity(person, household)
        Person.new(
          account: person.account,
          household: household,
          name: person.name,
          date_of_birth: person.date_of_birth,
          person_type: person.person_type,
          has_capacity: person.has_capacity,
          professional_title: person.professional_title
        )
      end
    end
  end
end
