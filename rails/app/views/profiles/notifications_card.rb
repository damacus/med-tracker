# frozen_string_literal: true

module Views
  module Profiles
    class NotificationsCard < Views::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::Routes

      attr_reader :person, :managed_grants

      def initialize(person:, preference: nil, managed_grants: nil)
        super()
        @person = person
        @preference = preference
        @managed_grants = managed_grants || []
      end

      def view_template
        Card(id: 'notifications-card') do
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
            role: 'status', aria: { live: 'polite', atomic: 'true' },
            data: { push_notification_target: 'status' }
          ) { t('profiles.notifications.checking_status') }
          render_push_action_buttons
        end
      end

      def render_push_action_buttons
        render ToggleGroup.new(
          type: :single,
          value: '0',
          variant: :outline,
          size: :sm,
          aria: { label: t('profiles.notifications.browser_title') }
        ) do |group|
          group.toggle_group_item(
            value: '1',
            class: 'min-h-11',
            data: {
              push_notification_target: 'subscribeButton',
              action: 'push-notification#subscribe'
            }
          ) { t('profiles.common.on') }
          group.toggle_group_item(
            value: '0',
            class: 'min-h-11',
            data: {
              push_notification_target: 'unsubscribeButton',
              action: 'push-notification#unsubscribe'
            }
          ) { t('profiles.common.off') }
        end
      end

      def render_test_button
        m3_button(
          type: 'button',
          variant: :outlined,
          size: :sm,
          class: 'w-full gap-2 font-bold',
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
          input(type: 'hidden', name: 'section', value: 'notifications')
          render_enabled_toggle
          render_category_toggles
          render NotificationDeliverySettings.new(preference:, managed_grants:)
          div(class: 'flex justify-end pt-2') do
            m3_button(
              type: 'submit',
              variant: :filled,
              size: :sm,
              class: 'font-bold'
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
          render_binary_toggle(
            name: 'notification_preference[enabled]',
            enabled: preference.enabled,
            label: t('profiles.notifications.enable_reminders')
          )
        end
      end

      def render_category_toggles
        div(class: 'space-y-3 border-t border-border pt-2') do
          render_section_header(t('notification_settings.categories.title'), t('profiles.notifications.categories_description'))
          div(class: 'space-y-3') do
            %i[dose_due_enabled missed_dose_enabled low_stock_enabled private_text_enabled].each do |category|
              render_category_toggle(category)
            end
          end
        end
      end

      def render_category_toggle(category)
        div(class: 'flex items-center justify-between gap-4') do
          div do
            label(class: 'text-sm font-medium text-foreground', for: "notification_preference_#{category}") { t("notification_settings.categories.#{category}.title") }
            p(class: 'mt-0.5 text-xs text-on-surface-variant') { t("notification_settings.categories.#{category}.description") }
          end
          render_binary_toggle(
            name: "notification_preference[#{category}]",
            enabled: preference.public_send(category),
            label: t("notification_settings.categories.#{category}.title")
          )
        end
      end

      def render_section_header(title, description)
        div(class: 'space-y-0.5') do
          h3(class: 'text-sm font-semibold text-foreground') { title }
          p(class: 'text-xs text-on-surface-variant') { description }
        end
      end

      def render_binary_toggle(name:, enabled:, label:)
        render ToggleGroup.new(
          type: :single,
          name:,
          value: enabled ? '1' : '0',
          variant: :outline,
          size: :sm,
          aria: { label: }
        ) do |group|
          group.toggle_group_item(value: '1', class: 'min-h-11') { t('profiles.common.on') }
          group.toggle_group_item(value: '0', class: 'min-h-11') { t('profiles.common.off') }
        end
      end
    end
  end
end
