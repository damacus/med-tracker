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

      def initialize(key:, title:, summary:, open: false)
        @key = key.to_sym
        @title = title
        @summary = summary
        @open = open
        super()
      end

      def view_template(&)
        render RubyUI::AccordionItem.new(
          id: section_id,
          open: @open,
          class: 'overflow-hidden rounded-shape-xl border border-outline-variant/70 bg-card shadow-elevation-2'
        ) do
          render_trigger
          render_content(&)
        end
      end

      private

      def render_trigger
        render RubyUI::AccordionTrigger.new(
          id: trigger_id,
          controls: content_id,
          expanded: @open,
          class: 'min-h-24 gap-4 px-5 py-5 text-left hover:bg-tertiary-container/30 focus-visible:outline-none ' \
                 'focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-inset',
          data: { testid: 'profile-section-trigger', profile_section: @key }
        ) do
          render_trigger_body
          render RubyUI::AccordionIcon.new
        end
      end

      def render_trigger_body
        div(class: 'flex min-w-0 flex-1 items-start gap-4', data: appearance_controller_data) do
          div(class: 'flex h-12 w-12 shrink-0 items-center justify-center rounded-full bg-primary-container text-on-primary-container') do
            render ICONS.fetch(@key).new(size: 24, aria_hidden: 'true')
          end
          div(class: 'min-w-0 flex-1') do
            h2(class: 'text-lg font-semibold text-foreground') { @title }
            div(class: 'mt-1 flex flex-wrap gap-x-3 gap-y-1 text-sm text-on-surface-variant') do
              @summary.each_with_index do |item, index|
                span(data: appearance_summary_data(index)) { item }
              end
            end
          end
        end
      end

      def render_content(&)
        render RubyUI::AccordionContent.new(
          id: content_id,
          labelledby: trigger_id,
          open: @open,
          class: 'border-t border-border/70'
        ) do
          div(class: 'space-y-5 p-4 sm:p-6', &)
        end
      end

      def section_id
        "profile-#{@key}-section"
      end

      def trigger_id
        "#{section_id}-trigger"
      end

      def content_id
        "#{section_id}-content"
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
