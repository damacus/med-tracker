# frozen_string_literal: true

require 'uri'

class AppSettings < ApplicationRecord
  has_paper_trail

  LOOKUP_SOURCE_KEYS = %w[
    imported_catalog
    local_nhs_dmd
    cached_open_products_facts
    open_products_facts
    curated_catalog
    nhs_dmd
    supplements
  ].freeze
  DEFAULT_LOOKUP_SOURCE_PRIORITY = LOOKUP_SOURCE_KEYS.freeze

  before_validation :normalize_medicine_lookup_source_priority

  validates :medicine_lookup_base_url, :medicine_lookup_token_url, presence: true
  validate :medicine_lookup_urls_are_https
  validate :medicine_lookup_source_priority_is_known

  # Always a single row. Access via AppSettings.instance, never instantiate directly.
  def self.instance
    first || create!(invite_only: default_invite_only?)
  end

  # Preserve the pre-settings safety behavior for deployments that have not
  # created the singleton row yet: once a household owner exists, registration
  # starts locked down unless an operator explicitly changes it later.
  def self.default_invite_only?
    return ActiveModel::Type::Boolean.new.cast(ENV.fetch('INVITE_ONLY')) if ENV.key?('INVITE_ONLY')

    HouseholdMembership.owner.active.exists?
  end

  # INVITE_ONLY env var takes precedence when set, letting ops pin the value
  # without touching the database.  When absent, the admin-configured DB value
  # is used.
  def self.invite_only?
    if ENV.key?('INVITE_ONLY')
      ActiveModel::Type::Boolean.new.cast(ENV.fetch('INVITE_ONLY'))
    else
      instance.invite_only
    end
  end

  # Returns :env when the value is locked by the environment variable,
  # :database otherwise.  Used by the admin UI to disable the toggle.
  def self.invite_only_source
    ENV.key?('INVITE_ONLY') ? :env : :database
  end

  def medicine_lookup_base_url
    self[:medicine_lookup_base_url].presence || NhsDmd::Client::BASE_URL
  end

  def medicine_lookup_token_url
    self[:medicine_lookup_token_url].presence || NhsDmd::Client::TOKEN_URL
  end

  def medicine_lookup_source_priority
    Array(self[:medicine_lookup_source_priority]).presence || DEFAULT_LOOKUP_SOURCE_PRIORITY
  end

  def lookup_source_priority_for(source_keys)
    known_source_keys = Array(source_keys)
    configured = medicine_lookup_source_priority & known_source_keys
    configured + (known_source_keys - configured)
  end

  private

  def normalize_medicine_lookup_source_priority
    self.medicine_lookup_source_priority = medicine_lookup_source_priority.map(&:to_s)
  end

  def medicine_lookup_urls_are_https
    %i[medicine_lookup_base_url medicine_lookup_token_url].each do |attribute|
      errors.add(attribute, 'must be an HTTPS URL') unless https_url?(public_send(attribute))
    end
  end

  def https_url?(value)
    uri = URI.parse(value.to_s)
    uri.is_a?(URI::HTTPS) && uri.host.present?
  rescue URI::InvalidURIError
    false
  end

  def medicine_lookup_source_priority_is_known
    return if (medicine_lookup_source_priority - LOOKUP_SOURCE_KEYS).empty?

    errors.add(:medicine_lookup_source_priority, 'contains unknown sources')
  end
end
