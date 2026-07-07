# frozen_string_literal: true

module Api
  class FreshPrivilegedAction
    TTL = 15.minutes

    def initialize(credential:)
      @credential = credential
    end

    def satisfied?
      return false unless credential.is_a?(ApiSession)
      return false unless credential.oidc_mfa_verified?
      return false if credential.mfa_verified_at.blank?

      credential.mfa_verified_at >= TTL.ago
    end

    private

    attr_reader :credential
  end
end
