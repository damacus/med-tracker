# frozen_string_literal: true

module Api
  module V1
    class PeopleController < BaseController
      def index
        authorize Person
        render_collection(policy_scope(Person), serializer: PersonSerializer, includes: %i[locations notification_preference])
      end

      def show
        person = policy_scope(Person).includes(:locations, :notification_preference).find(params.expect(:id))
        authorize person

        render_resource(person, serializer: PersonSerializer)
      end

      def create
        person = Person.new(person_params)
        person.household = current_household
        authorize person

        return render_validation_errors(person) unless persist_created_person(person)

        render_resource(person.reload, serializer: PersonSerializer, status: :created)
      end

      def update
        person = policy_scope(Person).includes(:locations, :notification_preference).find(params.expect(:id))
        authorize person

        return render_validation_errors(person) unless person.update(person_params)

        render_resource(person.reload, serializer: PersonSerializer)
      end

      private

      def person_params
        params.expect(person: %i[name date_of_birth email person_type has_capacity])
      end

      def grant_created_person_access(person)
        access_change.create_grant!(
          household: current_household,
          household_membership: current_membership,
          person: person,
          access_level: :manage,
          relationship_type: :family_member,
          granted_by_membership: current_membership
        )
      end

      def access_change
        @access_change ||= Households::AccessChange.new(
          actor_account: current_account,
          actor_membership: current_membership,
          request: request
        )
      end

      def persist_created_person(person)
        ActiveRecord::Base.transaction do
          if auto_assign_created_person_carer_relationship?(person)
            CareDelegation::Assign.new(
              carer: current_membership.person,
              patient: person,
              relationship_type: :family_member,
              granted_by_membership: current_membership
            ).call
          else
            person.save!
            grant_created_person_access(person)
          end
        end
        true
      rescue ActiveRecord::RecordInvalid => e
        person.errors.merge!(e.record.errors) unless e.record == person
        false
      rescue CareDelegation::Assign::Error => e
        person.errors.add(:base, e.message)
        false
      end

      def auto_assign_created_person_carer_relationship?(person)
        current_membership&.person.present? && (person.minor? || person.dependent_adult?)
      end
    end
  end
end
