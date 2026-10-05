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

        private

        def render_header
          div(class: 'flex flex-col md:flex-row md:items-end justify-between gap-6 mb-12') do
            div do
              m3_text(size: '2', weight: 'muted', class: 'uppercase tracking-widest mb-1 block font-bold') do
                Time.current.strftime('%A, %b %d')
              end
              m3_heading(level: 1, size: '8', class: 'font-extrabold tracking-tight') do
                t(@title_key)
              end
              m3_text(weight: 'muted', class: 'mt-2 block') { t(@subtitle_key) }
            end
            render RubyUI::Link.new(
              href: Rails.application.routes.url_helpers.platform_users_path,
              variant: :outlined,
              size: :lg
            ) { t('platform.users.title') }
          end
        end
      end
    end
  end
end
