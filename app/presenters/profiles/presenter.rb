# frozen_string_literal: true

module Profiles
  class Presenter
    SECTIONS = %w[profile security notifications advanced].freeze

    attr_reader :person, :account, :api_app_tokens, :new_api_app_token, :managed_notification_grants,
                :membership, :passkeys, :recovery_codes_count, :active_section, :notification_preference

    def self.build(person:, account:, household:, active_section: nil, new_api_app_token: nil)
      Loader.new(person:, account:, household:, active_section:, new_api_app_token:).call
    end

    def self.otp_enabled?(account)
      return false unless ActiveRecord::Base.connection.table_exists?('account_otp_keys')

      AccountOtpKey.exists?(id: account.id)
    end

    def self.recovery_codes_count(account)
      return 0 unless ActiveRecord::Base.connection.table_exists?('account_recovery_codes')

      AccountRecoveryCode.where(id: account.id).count
    end

    def initialize(person:, account:, active_section: nil, **data)
      @person = person
      @account = account
      @api_app_tokens = data.fetch(:api_app_tokens)
      @new_api_app_token = data[:new_api_app_token]
      @managed_notification_grants = data.fetch(:managed_notification_grants)
      @membership = data.fetch(:membership)
      @notification_preference = data.fetch(:notification_preference)
      @passkeys = data.fetch(:passkeys)
      @totp_enabled = data.fetch(:totp_enabled)
      @recovery_codes_count = data.fetch(:recovery_codes_count)
      @active_section = SECTIONS.include?(active_section.to_s) ? active_section.to_s : 'profile'
    end

    def totp_enabled?
      @totp_enabled
    end

    def profile_summary
      [person.name, account.email, I18n.t('profiles.appearance.modes.system')]
    end

    def security_summary
      two_factor = if totp_enabled?
                     I18n.t('profiles.sections.security.two_factor_on')
                   else
                     I18n.t('profiles.sections.security.two_factor_off')
                   end
      [I18n.t('profiles.sections.security.password_updated'), two_factor,
       I18n.t('profiles.sections.security.passkeys', count: passkeys.size)]
    end

    def notifications_summary
      state = notification_preference.enabled? ? I18n.t('profiles.common.on') : I18n.t('profiles.common.off')
      [I18n.t('profiles.sections.notifications.reminders', state:), enabled_category_summary]
    end

    def advanced_summary
      [I18n.t('profiles.sections.advanced.tokens', count: api_app_tokens.size),
       I18n.t('profiles.sections.advanced.exports')]
    end

    private

    def enabled_category_summary
      enabled = %i[dose_due_enabled missed_dose_enabled low_stock_enabled private_text_enabled].count do |category|
        notification_preference.public_send(category)
      end
      I18n.t('profiles.sections.notifications.categories', count: enabled)
    end

    class Loader
      def initialize(person:, account:, household:, active_section:, new_api_app_token:)
        @person = person
        @account = account
        @household = household
        @active_section = active_section
        @new_api_app_token = new_api_app_token
      end

      def call
        preload_avatar
        membership = @account.active_household_membership_for(@household)
        Presenter.new(
          person: @person,
          account: @account,
          active_section: @active_section,
          **loaded_data(membership)
        )
      end

      private

      def preload_avatar
        ActiveRecord::Associations::Preloader.new(
          records: [@person],
          associations: { avatar_attachment: :blob }
        ).call
      end

      def loaded_data(membership)
        {
          api_app_tokens: @account.api_app_tokens.active.order(created_at: :desc).to_a,
          new_api_app_token: @new_api_app_token,
          managed_notification_grants: ManagedNotificationGrantsQuery.new(membership:).call,
          membership:,
          notification_preference: @person.notification_preference || @person.build_notification_preference,
          passkeys: @account.account_webauthn_keys.order(created_at: :desc).to_a,
          totp_enabled: Presenter.otp_enabled?(@account),
          recovery_codes_count: Presenter.recovery_codes_count(@account)
        }
      end
    end
  end
end
