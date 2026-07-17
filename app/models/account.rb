# frozen_string_literal: true

class Account < ApplicationRecord
  include Rodauth::Rails.model

  has_paper_trail skip: %i[password_hash]

  WIZARD_VARIANTS = %w[fullpage modal slideover].freeze

  TIME_ZONE_NAMES = ActiveSupport::TimeZone.all.map(&:name).freeze

  store_accessor :preferences, :wizard_variant, :gravatar_enabled, :time_zone

  enum :status, { unverified: 1, verified: 2, closed: 3 }

  has_one :person, dependent: :nullify
  has_many :household_memberships, dependent: :destroy
  has_many :households, through: :household_memberships
  has_many :api_sessions, dependent: :destroy
  has_many :api_household_selection_grants, dependent: :destroy
  has_many :api_app_tokens, dependent: :destroy
  has_many :push_subscriptions, dependent: :destroy
  has_many :native_device_tokens, dependent: :destroy
  has_many :account_active_session_keys, dependent: :delete_all
  has_many :requested_household_exports, class_name: 'HouseholdExport', foreign_key: :requested_by_account_id,
                                         dependent: :restrict_with_error, inverse_of: :requested_by_account
  has_many :account_webauthn_keys, dependent: :destroy
  has_many :account_webauthn_user_ids, dependent: :destroy
  has_one :platform_admin, dependent: :destroy

  validates :email, presence: true, uniqueness: true
  validates :time_zone, inclusion: { in: TIME_ZONE_NAMES }, allow_blank: true

  def first_active_household_membership
    household_memberships.active.joins(:household).merge(Household.operational).includes(:household).order(:id).first
  end

  def first_active_household
    first_active_household_membership&.household
  end

  def active_household_membership_for(household)
    return if household.blank?

    return unless household.operational?

    household_memberships.active.find_by(household: household)
  end

  def wizard_variant
    variant = super
    WIZARD_VARIANTS.include?(variant) ? variant : 'fullpage'
  end

  def gravatar_enabled?
    !!ActiveModel::Type::Boolean.new.cast(gravatar_enabled)
  end

  def preferred_time_zone
    time_zone.presence || Rails.application.config.time_zone
  end
end
