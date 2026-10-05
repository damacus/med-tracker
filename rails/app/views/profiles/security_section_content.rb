# frozen_string_literal: true

module Views
  module Profiles
    class SecuritySectionContent < Views::Base
      def initialize(presenter:)
        super()
        @presenter = presenter
      end

      def view_template
        render AccountSecurityCard.new(account: @presenter.account)
        render TwoFactorCard.new(
          account: @presenter.account,
          passkeys: @presenter.passkeys,
          totp_enabled: @presenter.totp_enabled?,
          recovery_codes_count: @presenter.recovery_codes_count
        )
      end
    end
  end
end
