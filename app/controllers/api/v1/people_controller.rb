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

      def persist_created_person(person)
        People::Create.new(person: person, authorization: pundit_user, request: request).call
        true
      rescue ActiveRecord::RecordInvalid => e
        person.errors.merge!(e.record.errors) unless e.record == person
        false
      rescue CareDelegation::Assign::Error => e
        person.errors.add(:base, e.message)
        false
      end
    end
  end
end
