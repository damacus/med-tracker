# frozen_string_literal: true

module Views
  module Profiles
    class NotificationsCard < Views::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::Routes

      PERIOD_LABELS = {
        morning: 'profiles.notifications.periods.morning',
        afternoon: 'profiles.notifications.periods.afternoon',
        evening: 'profiles.notifications.periods.evening',
        night: 'profiles.notifications.periods.night'
      }.freeze

      attr_reader :person

      def initialize(person:)
        super()
        @person = person
      end

      def view_template
        Card do
          render CardHeader.new do
            render(CardTitle.new { t('profiles.notifications.title') })
            render(CardDescription.new { t('profiles.notifications.description') })
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
          render_vapid_meta
          render_section_header(
            t('profiles.notifications.browser_title'),
            t('profiles.notifications.browser_description')
          )
          render_push_status_box
        end
      end

      def render_vapid_meta
        vapid_public_key = ENV['VAPID_PUBLIC_KEY'] || Rails.application.credentials.dig(:vapid, :public_key)
        meta(name: 'vapid-public-key', content: vapid_public_key) if vapid_public_key
      end

      def render_push_status_box
        div(class: 'rounded-lg border border-border bg-secondary-container/60 p-4') do
          div(class: 'flex flex-col gap-4') do
            render_status_row
            render_test_button
          end
        end
      end

      def render_status_row
        div(class: 'flex items-center justify-between') do
          p(
            class: 'text-sm text-on-surface-variant',
            data: { push_notification_target: 'status' }
          ) { t('profiles.notifications.checking_status') }
          render_push_action_buttons
        end
      end

      def render_push_action_buttons
        div(class: 'flex gap-2 flex-shrink-0') do
          button(
            type: 'button',
            class: 'inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-primary text-white ' \
                   'font-medium text-sm hover:bg-primary/90 transition-colors',
            data: {
              push_notification_target: 'subscribeButton',
              action: 'push-notification#subscribe'
            },
            hidden: true
          ) { t('profiles.notifications.enable') }
          button(
            type: 'button',
            class: 'inline-flex items-center gap-1.5 rounded-lg border border-border px-3 py-1.5 ' \
                   'text-sm font-medium text-on-surface-variant transition-colors hover:bg-tertiary-container',
            data: {
              push_notification_target: 'unsubscribeButton',
              action: 'push-notification#unsubscribe'
            },
            hidden: true
          ) { t('profiles.notifications.disable') }
        end
      end

      def render_test_button
        button(
          type: 'button',
          class: 'w-full inline-flex items-center justify-center gap-2 rounded-xl border border-border bg-background px-4 py-2.5 ' \
                 'text-sm font-bold text-foreground transition-all hover:bg-tertiary-container hover:shadow-sm active:scale-[0.98]',
          data: {
            push_notification_target: 'testButton',
            action: 'push-notification#sendTest'
          },
          hidden: true
        ) do
          render Components::Icons::Send.new(size: 16)
          plain t('profiles.notifications.send_test_notification')
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
            ) { t('profiles.notifications.save') }
          end
        end
      end

      def render_enabled_toggle
        div(class: 'flex items-center justify-between') do
          div do
            p(class: 'text-sm font-medium text-foreground') { t('profiles.notifications.enable_reminders') }
            p(class: 'mt-0.5 text-xs text-on-surface-variant') { t('profiles.notifications.enable_reminders_description') }
          end
          div(class: 'flex items-center gap-2') do
            input(type: 'hidden', name: 'notification_preference[enabled]', value: '0')
            input(
              type: 'checkbox',
              name: 'notification_preference[enabled]',
              id: 'notification_preference_enabled',
              value: '1',
              checked: preference.enabled,
              class: 'h-4 w-4 rounded border-border bg-background text-primary focus:ring-primary'
            )
          end
        end
      end

      def render_time_slots
        div(class: 'space-y-2 border-t border-border pt-2') do
          render_section_header(t('profiles.notifications.reminder_times_title'), t('profiles.notifications.reminder_times_description'))
          div(class: 'grid grid-cols-2 gap-3 mt-2') do
            NotificationPreference::PERIODS.each { |period| render_time_slot(period) }
          end
        end
      end

      def render_time_slot(period)
        div do
          label(
            class: 'mb-1 block text-xs font-medium text-on-surface-variant',
            for: "notification_preference_#{period}_time"
          ) { t(PERIOD_LABELS[period]) }
          input(
            type: 'time',
            name: "notification_preference[#{period}_time]",
            id: "notification_preference_#{period}_time",
            value: preference.time_for_period(period)&.strftime('%H:%M'),
            class: 'w-full rounded-lg border border-border bg-background px-2 py-1.5 text-sm text-foreground ' \
                   'focus:outline-none focus:ring-2 focus:ring-primary/20 focus:border-primary'
          )
        end
      end

      def render_section_header(title, description)
        div(class: 'space-y-0.5') do
          h3(class: 'text-sm font-semibold text-foreground') { title }
          p(class: 'text-xs text-on-surface-variant') { description }
        end
      end
    end
  end
end
