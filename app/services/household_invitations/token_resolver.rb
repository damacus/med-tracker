# frozen_string_literal: true

module HouseholdInvitations
  class TokenResolver
    SETTING_NAME = 'med_tracker.current_invitation_token_digest'

    class << self
      def call(raw_token)
        new(raw_token).call
      end
    end

    def initialize(raw_token)
      @token_digest = HouseholdInvitation.digest(raw_token)
    end

    def call
      return if @token_digest.blank?

      ActiveRecord::Base.transaction(requires_new: true) do
        write_token_digest(@token_digest)
        HouseholdInvitation.pending.find_by(token_digest: @token_digest)
      ensure
        write_token_digest(nil)
      end
    end

    private

    def write_token_digest(value)
      ActiveRecord::Base.connection.execute(
        ActiveRecord::Base.sanitize_sql_array(
          ['SELECT set_config(?, ?, true)', SETTING_NAME, value.to_s]
        )
      )
    end
  end
end
