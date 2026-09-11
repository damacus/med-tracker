# frozen_string_literal: true

class OauthGrant < ApplicationRecord
  self.inheritance_column = nil

  belongs_to :account
  belongs_to :oauth_application
  belongs_to :household_membership, optional: true
  belongs_to :person, optional: true

  enum :client_kind, { integration: 'integration', mobile: 'mobile' }, validate: true
  validates :household_membership, :person, :permissions_version, presence: true, unless: :mobile?
  validates :authenticated_at, :last_used_at, presence: true, if: :mobile?
  validates :household_membership, :person, :permissions_version, absence: true, if: :mobile?
  validate :application_kind_matches

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

  def active_for_account?
    mobile? && revoked_at.nil? && scopes.to_s.split.include?('medtracker') &&
      login_active? && account_available?
  end

  def login_active?
    return false unless authenticated_at && last_used_at
    return false if last_used_at <= AuthenticationLifetime.inactivity_days.days.ago

    maximum_age = AuthenticationLifetime.maximum_age_days
    maximum_age.zero? || authenticated_at > maximum_age.days.ago
  end

  def refresh_expires_at
    deadlines = [last_used_at + AuthenticationLifetime.inactivity_days.days]
    maximum_age = AuthenticationLifetime.maximum_age_days
    deadlines << (authenticated_at + maximum_age.days) if maximum_age.positive?
    deadlines.min
  end

  def revoke!(audit_context: nil, action: 'revoked')
    with_lock do
      next if revoked_at

      update!(revoked_at: Time.current)
      Audit::VersionEvent.record!(item_type: self.class.name, item_id: id, event: "mobile_oauth.#{action}",
                                  object: { account_id: account_id, oauth_application_id: oauth_application_id },
                                  context: audit_context.to_h.merge(actor_account_id: account_id))
    end
  end

  def allows_fhir_read?(resource_type)
    granted_scopes = scopes.to_s.split
    granted_scopes.include?('patient/*.rs') || granted_scopes.include?('user/*.rs') ||
      granted_scopes.include?("patient/#{resource_type}.rs") || granted_scopes.include?("user/#{resource_type}.rs")
  end

  def patient_scoped?
    scopes.to_s.split.any? { |scope| scope.start_with?('patient/') }
  end

  private

  def account_available?
    account.verified? && account.person&.user&.active? && !ApiAuthState.locked_out?(account)
  end

  def application_kind_matches
    return if oauth_application.nil? || oauth_application.client_kind == client_kind

    errors.add(:client_kind, 'must match the registered application')
  end
end
