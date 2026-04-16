# frozen_string_literal: true

module Components
  module NotificationSettings
    class FormView < Components::Base
      PERIOD_LABELS = {
        morning: 'morning',
        afternoon: 'afternoon',
        evening: 'evening',
        night: 'night'
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
          h1(class: 'text-3xl font-black text-foreground') { t('notification_settings.title') }
          p(class: 'text-on-surface-variant mt-2') { t('notification_settings.description') }
        end
      end

      def render_push_subscription_section
        div(
          class: 'mb-8 p-6 rounded-2xl border border-border bg-secondary-container',
          data: { controller: 'push-notification' }
        ) do
          # VAPID key fallback if not in layout
          vapid_public_key = ENV['VAPID_PUBLIC_KEY'] || Rails.application.credentials.dig(:vapid, :public_key)
          meta(name: 'vapid-public-key', content: vapid_public_key) if vapid_public_key

          div(class: 'flex items-center gap-3 mb-4') do
            render Icons::Bell.new(size: 24)
            h2(class: 'text-lg font-bold text-foreground') { t('notification_settings.browser.title') }
          end
          p(
            class: 'text-sm text-on-surface-variant mb-4',
            data: { push_notification_target: 'status' }
          ) { t('notification_settings.browser.status') }
          div(class: 'flex flex-col gap-3') do
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
                plain t('notification_settings.browser.enable')
              end
              button(
                type: 'button',
                class: 'inline-flex items-center gap-2 px-4 py-2 rounded-xl border border-border ' \
                       'text-on-surface-variant font-bold text-sm hover:bg-tertiary-container transition-colors',
                data: {
                  push_notification_target: 'unsubscribeButton',
                  action: 'push-notification#unsubscribe'
                },
                hidden: true
              ) do
                t('notification_settings.browser.disable')
              end
            end

            button(
              type: 'button',
              class: 'w-full inline-flex items-center justify-center gap-2 rounded-xl border border-border ' \
                     'bg-card px-4 py-2.5 text-sm font-bold text-foreground transition-all ' \
                     'hover:bg-tertiary-container ' \
                     'hover:shadow-sm active:scale-[0.98]',
              data: {
                push_notification_target: 'testButton',
                action: 'push-notification#sendTest'
              },
              hidden: true
            ) do
              render Components::Icons::Send.new(size: 16)
              plain t('notification_settings.browser.test')
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
        div(class: 'p-6 rounded-2xl border border-border') do
          div(class: 'flex items-center justify-between') do
            div do
              label(class: 'font-bold text-foreground', for: 'notification_preference_enabled') do
                t('notification_settings.reminders.enable')
              end
              p(class: 'text-sm text-on-surface-variant mt-1') { t('notification_settings.reminders.description') }
            end
            input(type: 'hidden', name: 'notification_preference[enabled]', value: '0')
            input(
              type: 'checkbox',
              name: 'notification_preference[enabled]',
              id: 'notification_preference_enabled',
              value: '1',
              checked: preference.enabled,
              class: 'h-5 w-5 rounded border-border text-primary focus:ring-primary'
            )
          end
        end
      end

      def render_time_slots(_f)
        div(class: 'p-6 rounded-2xl border border-border space-y-4') do
          h2(class: 'font-bold text-foreground mb-4') { t('notification_settings.reminders.title') }
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
          ) { t("notification_settings.period_labels.#{PERIOD_LABELS.fetch(period)}") }
          input(
            type: 'time',
            name: "notification_preference[#{period}_time]",
            id: "notification_preference_#{period}_time",
            value: time_value&.strftime('%H:%M'),
            class: 'rounded-xl border border-border px-3 py-2 text-sm focus:outline-none ' \
                   'focus:ring-2 focus:ring-primary/20 focus:border-primary'
          )
        end
      end

      def render_actions
        div(class: 'flex gap-3 justify-end pt-4') do
          a(
            href: root_path,
            class: 'inline-flex items-center px-4 py-2 rounded-xl border border-border ' \
                   'text-on-surface-variant font-bold text-sm hover:bg-tertiary-container transition-colors no-underline'
          ) { t('notification_settings.actions.cancel') }
          button(
            type: 'submit',
            class: 'inline-flex items-center px-4 py-2 rounded-xl bg-primary text-white font-bold ' \
                   'text-sm hover:bg-primary/90 transition-colors'
          ) { t('notification_settings.actions.save') }
        end
      end
    end
  end
end