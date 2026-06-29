# frozen_string_literal: true

module Components
  module Platform
    module Settings
      class ShowView < Components::Admin::Settings::ShowView
        def initialize(settings:)
          super(
            settings: settings,
            form_url: Rails.application.routes.url_helpers.platform_settings_path,
            title_key: 'platform.settings.title',
            subtitle_key: 'platform.settings.subtitle',
            dom_id: 'platform_settings'
          )
        end
      end
    end
  end
end
