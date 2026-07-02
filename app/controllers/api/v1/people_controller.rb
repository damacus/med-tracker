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
        assign_created_person_carer_relationship(person)

        return render_validation_errors(person) unless person.save

        grant_created_person_access(person)
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
        current_household.person_access_grants.find_or_create_by!(
          household_membership: current_membership,
          person: person
        ) do |grant|
          grant.access_level = :manage
          grant.relationship_type = :family_member
          grant.granted_by_membership = current_membership
        end
      end

      def assign_created_person_carer_relationship(person)
        return unless current_membership&.person.present? && (person.minor? || person.dependent_adult?)

        person.carer_relationships.build(
          carer: current_membership.person,
          relationship_type: :family_member,
          active: true
        )
      end
    end
  end
end
