# frozen_string_literal: true

module Components
  module Admin
    module Settings
      class ShowView < Components::Base
        include Phlex::Rails::Helpers::FormWith

        def initialize(settings:)
          @settings = settings
          @env_controlled = AppSettings.invite_only_source == :env
        end

        def view_template
          div(id: 'admin_settings', class: 'container mx-auto px-4 py-8 pb-24 md:pb-8 max-w-6xl space-y-8') do
            render_header
            render_access_section
          end
        end

        private

        def render_header
          div(class: 'flex flex-col md:flex-row md:items-end justify-between gap-6 mb-12') do
            div do
              m3_text(size: '2', weight: 'muted', class: 'uppercase tracking-widest mb-1 block font-bold') do
                Time.current.strftime('%A, %b %d')
              end
              m3_heading(level: 1, size: '8', class: 'font-extrabold tracking-tight') do
                t('admin.settings.title')
              end
              m3_text(weight: 'muted', class: 'mt-2 block') { t('admin.settings.subtitle') }
            end
          end
        end

        def render_access_section
          div(class: 'max-w-2xl mx-auto w-full') do
            m3_card(class: 'overflow-hidden border-none shadow-2xl rounded-[2.5rem] bg-card') do
              div(class: 'p-10 space-y-8') do
                render_section_heading
                render_env_notice if @env_controlled
                render_form
              end
            end
          end
        end

        def render_section_heading
          div do
            m3_heading(level: 2, size: '5', class: 'font-bold tracking-tight') do
              t('admin.settings.access.title')
            end
            m3_text(weight: 'muted', class: 'mt-1 block text-sm') { t('admin.settings.access.subtitle') }
          end
        end

        def render_env_notice
          classes = [
            'flex items-start gap-3 rounded-2xl border border-amber-200 bg-amber-50',
            'dark:border-amber-800 dark:bg-amber-950/30 px-5 py-4'
          ].join(' ')

          div(class: classes) do
            span(class: 'mt-0.5 text-amber-600 dark:text-amber-400 shrink-0') do
              # info icon
              svg(xmlns: 'http://www.w3.org/2000/svg', width: '16', height: '16', viewbox: '0 0 24 24',
                  fill: 'none', stroke: 'currentColor', stroke_width: '2',
                  stroke_linecap: 'round', stroke_linejoin: 'round') do |s|
                s.circle(cx: '12', cy: '12', r: '10')
                s.path(d: 'M12 16v-4M12 8h.01')
              end
            end
            div(class: 'space-y-1') do
              p(class: 'text-sm font-semibold text-amber-800 dark:text-amber-300') do
                t('admin.settings.env_override.title')
              end
              p(class: 'text-xs text-amber-700 dark:text-amber-400') do
                t('admin.settings.env_override.body', var: 'INVITE_ONLY',
                                                      value: ENV.fetch('INVITE_ONLY'))
              end
            end
          end
        end

        def render_form
          form_with(url: admin_settings_path, method: :patch, class: 'space-y-6') do
            render_invite_only_toggle
            render_save_button unless @env_controlled
          end
        end

        def render_invite_only_toggle
          classes = [
            'flex items-start justify-between gap-6 rounded-2xl border border-border p-6',
            ('opacity-60' if @env_controlled)
          ].compact.join(' ')

          div(class: classes) do
            div(class: 'space-y-1') do
              label(for: 'app_settings_invite_only', class: 'font-semibold text-foreground cursor-pointer') do
                t('admin.settings.access.invite_only.label')
              end
              p(class: 'text-sm text-on-surface-variant') do
                t('admin.settings.access.invite_only.hint')
              end
            end
            div(class: 'flex items-center shrink-0 pt-1') do
              input(type: 'hidden', name: 'app_settings[invite_only]', value: '0')
              input(
                type: 'checkbox',
                id: 'app_settings_invite_only',
                name: 'app_settings[invite_only]',
                value: '1',
                checked: AppSettings.invite_only?,
                disabled: @env_controlled,
                class: 'h-5 w-5 rounded border-border text-primary focus:ring-primary disabled:cursor-not-allowed'
              )
            end
          end
        end

        def render_save_button
          div(class: 'flex justify-end') do
            m3_button(type: :submit, variant: :filled, size: :lg,
                      class: 'px-8 rounded-2xl shadow-lg shadow-primary/20') do
              t('admin.settings.save')
            end
          end
        end
      end
    end
  end
end
