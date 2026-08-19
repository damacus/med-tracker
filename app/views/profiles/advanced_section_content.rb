# frozen_string_literal: true

module Views
  module Profiles
    class AdvancedSectionContent < Views::Base
      def initialize(presenter:)
        super()
        @presenter = presenter
      end

      def view_template
        render RubyUI::Accordion.new(class: 'space-y-3') do
          render_api_tokens
          render_data_backup
          render_experiments
          render_system_information
        end
        render DangerZoneCard.new
      end

      private

      def render_api_tokens
        render_nested_section(
          key: :api_tokens,
          title: t('profiles.api_tokens.title'),
          description: t('profiles.api_tokens.description')
        ) do
          render ApiTokensCard.new(
            account: @presenter.account,
            membership: @presenter.membership,
            api_app_tokens: @presenter.api_app_tokens,
            new_api_app_token: @presenter.new_api_app_token
          )
        end
      end

      def render_data_backup
        render_nested_section(
          key: :data_backup,
          title: t('profiles.sections.advanced.data_backup'),
          description: t('profiles.sections.advanced.data_backup_description')
        ) { render DataExportsCard.new }
      end

      def render_experiments
        render_nested_section(
          key: :experiments,
          title: t('profiles.sections.advanced.experiments'),
          description: t('profiles.sections.advanced.experiments_description')
        ) { render ExperimentsCard.new(account: @presenter.account) }
      end

      def render_system_information
        render_nested_section(
          key: :system_information,
          title: t('profiles.version_info.title'),
          description: t('profiles.version_info.description')
        ) { render VersionInfo.new }
      end

      def render_nested_section(key:, title:, description:, &content)
        render NestedSection.new(key:, title:, description:, &content)
      end
    end
  end
end
