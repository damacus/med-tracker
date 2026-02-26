# frozen_string_literal: true

module Components
  module NotificationSettings
    class FormView < Components::Base
      PERIOD_LABELS = {
        morning: 'Morning',
        afternoon: 'Afternoon',
        evening: 'Evening',
        night: 'Night'
      }.freeze

      attr_reader :preference

      def initialize(preference:)
        @preference = preference
        super()
      end

      def view_template
        div(class: 'container mx-auto px-4 py-8 max-w-2xl') do
          render_header
          render_push_subscription_section
          render_form
        end
      end

      private

      def render_header
        div(class: 'mb-8') do
          h1(class: 'text-3xl font-black text-foreground') { 'Notification Settings' }
          p(class: 'text-slate-500 mt-2') { 'Manage your medication reminder notifications.' }
        end
      end

      def render_push_subscription_section
        div(
          class: 'mb-8 p-6 rounded-2xl border border-slate-200 bg-slate-50/50',
          data: { controller: 'push-notification' }
        ) do
          div(class: 'flex items-center gap-3 mb-4') do
            render Icons::Bell.new(size: 24)
            h2(class: 'text-lg font-bold text-foreground') { 'Browser Notifications' }
          end
          p(
            class: 'text-sm text-slate-500 mb-4',
            data: { push_notification_target: 'status' }
          ) { 'Checking notification status...' }
          div(class: 'flex gap-3') do
            button(
              type: 'button',
              class: 'inline-flex items-center gap-2 px-4 py-2 rounded-xl bg-primary text-white font-bold ' \
                     'text-sm hover:bg-primary/90 transition-colors',
              data: {
                push_notification_target: 'subscribeButton',
                action: 'push-notification#subscribe'
              },
              hidden: true
            ) do
              render Icons::Bell.new(size: 16)
              plain 'Enable Notifications'
            end
            button(
              type: 'button',
              class: 'inline-flex items-center gap-2 px-4 py-2 rounded-xl border border-slate-200 ' \
                     'text-slate-600 font-bold text-sm hover:bg-slate-100 transition-colors',
              data: {
                push_notification_target: 'unsubscribeButton',
                action: 'push-notification#unsubscribe'
              },
              hidden: true
            ) do
              'Disable Notifications'
            end
          end
        end
      end

      def render_form
        form_with(model: preference, url: notification_preference_path, method: :patch, class: 'space-y-6') do |f|
          render_enabled_toggle(f)
          render_time_slots(f)
          render_actions
        end
      end

      def render_enabled_toggle(_f)
        div(class: 'p-6 rounded-2xl border border-slate-200') do
          div(class: 'flex items-center justify-between') do
            div do
              label(class: 'font-bold text-foreground', for: 'notification_preference_enabled') do
                'Enable Reminders'
              end
              p(class: 'text-sm text-slate-500 mt-1') { 'Send medication reminders at scheduled times.' }
            end
            input(type: 'hidden', name: 'notification_preference[enabled]', value: '0')
            input(
              type: 'checkbox',
              name: 'notification_preference[enabled]',
              id: 'notification_preference_enabled',
              value: '1',
              checked: preference.enabled,
              class: 'h-5 w-5 rounded border-slate-300 text-primary focus:ring-primary'
            )
          end
        end
      end

      def render_time_slots(_f)
        div(class: 'p-6 rounded-2xl border border-slate-200 space-y-4') do
          h2(class: 'font-bold text-foreground mb-4') { 'Reminder Times' }
          NotificationPreference::PERIODS.each do |period|
            render_time_slot(period)
          end
        end
      end

      def render_time_slot(period)
        time_value = preference.time_for_period(period)
        div(class: 'flex items-center justify-between gap-4') do
          label(
            class: 'font-medium text-sm text-foreground w-24',
            for: "notification_preference_#{period}_time"
          ) { PERIOD_LABELS[period] }
          input(
            type: 'time',
            name: "notification_preference[#{period}_time]",
            id: "notification_preference_#{period}_time",
            value: time_value&.strftime('%H:%M'),
            class: 'rounded-xl border border-slate-200 px-3 py-2 text-sm focus:outline-none ' \
                   'focus:ring-2 focus:ring-primary/20 focus:border-primary'
          )
        end
      end

      def render_actions
        div(class: 'flex gap-3 justify-end pt-4') do
          a(
            href: root_path,
            class: 'inline-flex items-center px-4 py-2 rounded-xl border border-slate-200 ' \
                   'text-slate-600 font-bold text-sm hover:bg-slate-100 transition-colors no-underline'
          ) { 'Cancel' }
          button(
            type: 'submit',
            class: 'inline-flex items-center px-4 py-2 rounded-xl bg-primary text-white font-bold ' \
                   'text-sm hover:bg-primary/90 transition-colors'
          ) { 'Save Settings' }
        end
      end
    end
  end
end
