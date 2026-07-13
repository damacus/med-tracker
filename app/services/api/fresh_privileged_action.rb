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
      mfa_verified_at = credential.mfa_verified_at
      return false if mfa_verified_at.blank?

      mfa_verified_at >= TTL.ago
    end

    private

    attr_reader :credential
  end
end
