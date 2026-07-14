# frozen_string_literal: true

class OauthGrant < ApplicationRecord
  self.inheritance_column = nil

  belongs_to :account
  belongs_to :oauth_application
  belongs_to :household_membership
  belongs_to :person

  scope :active, -> { where(revoked_at: nil).where(expires_in: Time.current..) }

  class << self
    def lookup_by_access_token(token)
      active.find_by(token_hash: digest(token))
    end

    def digest(token)
      Base64.urlsafe_encode64(Digest::SHA256.digest(token.to_s))
    end
  end

  def active_for_membership?
    household_membership&.active? && household_membership.household&.operational? &&
      permissions_version == household_membership.permissions_version
  end

  def touch_last_used!
    update!(last_used_at: Time.current)
  end

  def allows_fhir_read?(resource_type)
    granted_scopes = scopes.to_s.split
    granted_scopes.include?('patient/*.rs') || granted_scopes.include?('user/*.rs') ||
      granted_scopes.include?("patient/#{resource_type}.rs") || granted_scopes.include?("user/#{resource_type}.rs")
  end

  def patient_scoped?
    scopes.to_s.split.any? { |scope| scope.start_with?('patient/') }
  end
end
