# frozen_string_literal: true

module Views
  module Profiles
    class Section < Components::Base
      ICONS = {
        profile: Components::Icons::User,
        security: Components::Icons::Lock,
        notifications: Components::Icons::Bell,
        advanced: Components::Icons::Settings
      }.freeze

      def initialize(key:, title:, summary:)
        @key = key.to_sym
        @title = title
        @summary = summary
        super()
      end

      def view_template(&)
        section(
          id: panel_id,
          role: 'region',
          aria: { labelledby: tab_id },
          data: { testid: 'profile-section-panel' },
          class: 'overflow-hidden rounded-shape-xl border border-outline-variant/70 bg-card shadow-elevation-2'
        ) do
          render_header
          div(class: 'border-t border-border/70 p-4 sm:p-6', &)
        end
      end

      private

      def render_header
        div(class: 'flex items-start gap-4 px-5 py-5 sm:px-6', data: appearance_controller_data) do
          div(class: 'flex h-11 w-11 shrink-0 items-center justify-center rounded-full bg-primary-container text-on-primary-container') do
            render ICONS.fetch(@key).new(size: 22, aria_hidden: 'true')
          end
          div(class: 'min-w-0 flex-1') do
            h2(class: 'text-xl font-semibold tracking-tight text-foreground') { @title }
            render_summary
          end
        end
      end

      def render_summary
        div(
          class: 'mt-2 flex flex-wrap gap-2',
          data: { profile_section_summary: @key }
        ) do
          @summary.each_with_index do |item, index|
            render Badge.new(
              variant: :outline,
              size: :sm,
              data: appearance_summary_data(index)
            ) { item }
          end
        end
      end

      def panel_id
        "profile-#{@key}-panel"
      end

      def tab_id
        "profile-tab-#{@key}"
      end

      def appearance_summary_data(index)
        return {} unless @key == :profile && index == @summary.length - 1

        {
          appearance_summary: true,
          light_label: t('profiles.appearance.modes.light'),
          dark_label: t('profiles.appearance.modes.dark'),
          system_label: t('profiles.appearance.modes.system')
        }
      end

      def appearance_controller_data
        @key == :profile ? { controller: 'appearance' } : {}
      end
    end
  end
end
