# frozen_string_literal: true

module Views
  module Profiles
    class ManagedNotificationPeople < Views::Base
      def initialize(grants:)
        super()
        @grants = grants
      end

      def view_template
        div(class: 'space-y-3 border-t border-border pt-2', data: { testid: 'managed-notification-people' }) do
          div(class: 'space-y-0.5') do
            h3(class: 'text-sm font-semibold text-foreground') { t('profiles.notifications.managed_people.title') }
            p(class: 'text-xs text-on-surface-variant') do
              t('profiles.notifications.managed_people.description')
            end
          end
          input(type: 'hidden', name: 'notification_preference[managed_person_ids][]', value: '')
          div(class: 'space-y-2') do
            grants.each { |grant| render_managed_person(grant) }
          end
        end
      end

      private

      attr_reader :grants

      def render_managed_person(grant)
        managed_person = grant.person
        div(class: 'flex min-h-11 items-center justify-between gap-4 rounded-shape-lg bg-surface-container px-3 py-2') do
          span(class: 'text-sm font-medium text-foreground') { managed_person.name }
          if managed_person.minor? || managed_person.dependent_adult?
            m3_badge(variant: :tonal) { t('profiles.notifications.managed_people.automatic') }
          else
            render_managed_adult_toggle(grant)
          end
        end
      end

      def render_managed_adult_toggle(grant)
        managed_person = grant.person
        input_id = "notification_preference_managed_person_#{managed_person.id}"
        label(for: input_id, class: 'flex min-h-11 cursor-pointer items-center gap-3 text-xs font-medium text-foreground') do
          span { t('profiles.notifications.managed_people.notify_me') }
          input(
            type: 'checkbox',
            name: 'notification_preference[managed_person_ids][]',
            id: input_id,
            value: managed_person.id,
            checked: grant.missed_dose_notifications_enabled?,
            aria_label: t('profiles.notifications.managed_people.toggle_label', name: managed_person.name),
            class: 'h-5 w-5 rounded border-border bg-background text-primary focus:ring-primary'
          )
        end
      end
    end
  end
end
