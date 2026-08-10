# frozen_string_literal: true

class OauthApplication < ApplicationRecord
  TOKEN_ENDPOINT_AUTH_METHODS = [
    'none',
    'client_secret_basic',
    'client_secret_post',
    'client_secret_basic client_secret_post'
  ].freeze
  CONFIDENTIAL_AUTH_METHODS = %w[client_secret_basic client_secret_post].freeze

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
  validates :token_endpoint_auth_method, inclusion: { in: TOKEN_ENDPOINT_AUTH_METHODS }
  validate :redirect_uri_uses_https
  validate :scopes_are_supported
  validate :token_endpoint_auth_method_matches_secret

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

  def token_endpoint_auth_method_matches_secret
    return if token_endpoint_auth_method.blank?

    methods = token_endpoint_auth_method.split
    valid = if client_secret.present? || client_secret_hash.present?
              methods.any? && (methods - CONFIDENTIAL_AUTH_METHODS).empty?
            else
              methods == ['none']
            end
    errors.add(:token_endpoint_auth_method, 'does not match the client secret configuration') unless valid
  end
end
