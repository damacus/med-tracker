module Api
  module V1
    class ProfileSerializer
      def initialize(person:, account:)
        @person = person
        @account = account
      end

      def as_json(*)
        { person_id: @person.id.to_s, account_id: @account.id.to_s,
          date_of_birth: @person.date_of_birth&.iso8601, time_zone: @account.preferred_time_zone,
          gravatar_enabled: @account.gravatar_enabled?, mobile_shortcuts: @account.preferred_mobile_shortcuts,
          avatar_attached: @person.avatar.attached? }
      end
    end
  end
end
