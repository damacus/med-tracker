# frozen_string_literal: true

module Views
  module Profiles
    class NotificationDeliverySettings < Views::Base
      PERIOD_LABELS = {
        morning: 'profiles.notifications.periods.morning', afternoon: 'profiles.notifications.periods.afternoon',
        evening: 'profiles.notifications.periods.evening', night: 'profiles.notifications.periods.night'
      }.freeze

      def initialize(preference:, managed_grants:)
        super()
        @preference = preference
        @managed_grants = managed_grants
      end

      def view_template
        render RubyUI::Accordion.new(class: 'space-y-3 border-t border-border pt-4') do
          render_managed_people if @managed_grants.any?
          render_nested_row(
            id: 'notification-delivery-times',
            title: t('profiles.notifications.reminder_times_title'),
            description: t('profiles.notifications.reminder_times_description')
          ) { render_time_slots }
        end
      end

      private

      def render_managed_people
        render_nested_row(
          id: 'notification-managed-people',
          title: t('profiles.notifications.managed_people.title'),
          description: t('profiles.notifications.managed_people.description')
        ) { render ManagedNotificationPeople.new(grants: @managed_grants) }
      end

      def render_nested_row(id:, title:, description:, &content)
        render RubyUI::AccordionItem.new(class: 'rounded-shape-lg border border-border/70 bg-popover') do
          render_nested_trigger(id:, title:, description:)
          render RubyUI::AccordionContent.new(
            id: "#{id}-content",
            labelledby: "#{id}-trigger",
            open: false,
            class: 'px-4 pb-4',
            &content
          )
        end
      end

      def render_nested_trigger(id:, title:, description:)
        render RubyUI::AccordionTrigger.new(
          controls: "#{id}-content",
          expanded: false,
          id: "#{id}-trigger",
          class: 'flex min-h-14 w-full items-center justify-between gap-3 px-4 py-3 text-left'
        ) do
          div do
            p(class: 'text-sm font-semibold text-foreground') { title }
            p(class: 'mt-1 text-xs text-on-surface-variant') { description }
          end
          render RubyUI::AccordionIcon.new
        end
      end

      def render_time_slots
        div(class: 'grid grid-cols-1 gap-3 sm:grid-cols-2') do
          NotificationPreference::PERIODS.each { |period| render_time_slot(period) }
        end
      end

      def render_time_slot(period)
        div do
          label(
            class: 'mb-1 block text-xs font-medium text-on-surface-variant',
            for: "notification_preference_#{period}_time"
          ) { t(PERIOD_LABELS.fetch(period)) }
          input(
            type: 'time',
            name: "notification_preference[#{period}_time]",
            id: "notification_preference_#{period}_time",
            value: @preference.time_for_period(period)&.strftime('%H:%M'),
            class: 'w-full rounded-lg border border-border bg-background px-2 py-1.5 text-sm text-foreground ' \
                   'focus:outline-none focus:ring-2 focus:ring-primary/20 focus:border-primary'
          )
        end
      end
    end
  end
end
