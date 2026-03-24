# frozen_string_literal: true

module Api
  module V1
    class MeSerializer
      def initialize(user)
        @user = user
      end

      def as_json(*)
        {
          id: user.id,
          email_address: user.email_address,
          role: user.role,
          active: user.active
        }.merge(person_data).merge(account_data)
      end

      private

      attr_reader :user

      def person_data
        { person: PersonSerializer.new(user.person).as_json }
      end

      def account_data
        {
          account: {
            id: user.person.account.id,
            email: user.person.account.email,
            status: user.person.account.status
          }
        }
      end
    end
  end
end
