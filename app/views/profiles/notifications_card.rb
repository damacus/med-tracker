# frozen_string_literal: true

module Views
  module Profiles
    class NotificationsCard < Views::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::Routes

      PERIOD_LABELS = {
        morning: 'Morning',
        afternoon: 'Afternoon',
        evening: 'Evening',
        night: 'Night'
      }.freeze

      attr_reader :person

      def initialize(person:)
        super()
        @person = person
      end

      def view_template
        render Card.new do
          render CardHeader.new do
            render(CardTitle.new { 'Notifications' })
            render(CardDescription.new { 'Manage medication reminders and browser notifications' })
          end
          render CardContent.new(class: 'space-y-6') do
            render_push_subscription_section
            render_preferences_form
          end
        end
      end

      private

      def preference
        @preference ||= person.notification_preference || person.build_notification_preference
      end

      def render_push_subscription_section
        div(
          class: 'space-y-3',
          data: { controller: 'push-notification' }
        ) do
          render_section_header(
            'Browser Notifications',
            'Allow this device to receive medication reminders'
          )
          div(class: 'flex items-center justify-between p-3 border border-slate-200 rounded-lg bg-slate-50') do
            p(
              class: 'text-sm text-slate-600',
              data: { push_notification_target: 'status' }
            ) { 'Checking notification status...' }
            div(class: 'flex gap-2 ml-4 flex-shrink-0') do
              button(
                type: 'button',
                class: 'inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-primary text-white ' \
                       'font-medium text-sm hover:bg-primary/90 transition-colors',
                data: {
                  push_notification_target: 'subscribeButton',
                  action: 'push-notification#subscribe'
                },
                hidden: true
              ) { 'Enable' }
              button(
                type: 'button',
                class: 'inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg border border-slate-200 ' \
                       'text-slate-600 font-medium text-sm hover:bg-slate-100 transition-colors',
                data: {
                  push_notification_target: 'unsubscribeButton',
                  action: 'push-notification#unsubscribe'
                },
                hidden: true
              ) { 'Disable' }
            end
          end
        end
      end

      def render_preferences_form
        form_with(
          model: preference,
          url: notification_preference_path,
          method: :patch,
          class: 'space-y-4'
        ) do |_f|
          render_enabled_toggle
          render_time_slots
          div(class: 'flex justify-end pt-2') do
            button(
              type: 'submit',
              class: 'inline-flex items-center px-4 py-2 rounded-xl bg-primary text-white font-bold ' \
                     'text-sm hover:bg-primary/90 transition-colors'
            ) { 'Save' }
          end
        end
      end

      def render_enabled_toggle
        div(class: 'flex items-center justify-between') do
          div do
            p(class: 'text-sm font-medium text-slate-900') { 'Enable reminders' }
            p(class: 'text-xs text-slate-500 mt-0.5') { 'Send notifications at the times below' }
          end
          div(class: 'flex items-center gap-2') do
            input(type: 'hidden', name: 'notification_preference[enabled]', value: '0')
            input(
              type: 'checkbox',
              name: 'notification_preference[enabled]',
              id: 'notification_preference_enabled',
              value: '1',
              checked: preference.enabled,
              class: 'h-4 w-4 rounded border-slate-300 text-primary focus:ring-primary'
            )
          end
        end
      end

      def render_time_slots
        div(class: 'space-y-2 pt-2 border-t border-slate-100') do
          render_section_header('Reminder times', 'Set when you want to receive reminders each day')
          div(class: 'grid grid-cols-2 gap-3 mt-2') do
            NotificationPreference::PERIODS.each { |period| render_time_slot(period) }
          end
        end
      end

      def render_time_slot(period)
        div do
          label(
            class: 'block text-xs font-medium text-slate-500 mb-1',
            for: "notification_preference_#{period}_time"
          ) { PERIOD_LABELS[period] }
          input(
            type: 'time',
            name: "notification_preference[#{period}_time]",
            id: "notification_preference_#{period}_time",
            value: preference.time_for_period(period)&.strftime('%H:%M'),
            class: 'w-full rounded-lg border border-slate-200 px-2 py-1.5 text-sm ' \
                   'focus:outline-none focus:ring-2 focus:ring-primary/20 focus:border-primary'
          )
        end
      end

      def render_section_header(title, description)
        div(class: 'space-y-0.5') do
          h3(class: 'text-sm font-semibold text-slate-900') { title }
          p(class: 'text-xs text-slate-600') { description }
        end
      end
    end
  end
end
