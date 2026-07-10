# frozen_string_literal: true

class OauthApplication < ApplicationRecord
  SUPPORTED_SCOPES = %w[
    launch/patient
    offline_access
    online_access
    patient/*.rs
    patient/Patient.rs
    patient/Medication.rs
    patient/MedicationRequest.rs
    patient/MedicationStatement.rs
    patient/MedicationAdministration.rs
    user/*.rs
  ].freeze

  belongs_to :account, optional: true
  has_many :oauth_grants, dependent: :destroy

  validates :name, :client_id, :redirect_uri, :scopes, presence: true
  validate :redirect_uri_uses_https
  validate :scopes_are_supported

  private

  def redirect_uri_uses_https
    redirect_uri.to_s.split.each do |value|
      uri = URI.parse(value)
      errors.add(:redirect_uri, 'must use HTTPS') unless uri.is_a?(URI::HTTPS) && uri.host.present?
    rescue URI::InvalidURIError
      errors.add(:redirect_uri, 'must be a valid HTTPS URI')
    end
  end

  def scopes_are_supported
    unsupported = scopes.to_s.split - SUPPORTED_SCOPES
    errors.add(:scopes, "include unsupported values: #{unsupported.join(', ')}") if unsupported.any?
  end
end
