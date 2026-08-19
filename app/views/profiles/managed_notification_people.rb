# frozen_string_literal: true

module Views
  module Profiles
    class ManagedNotificationPeople < Views::Base
      def initialize(grants:)
        super()
        @grants = grants
      end

      def view_template
        div(class: 'space-y-3', data: { testid: 'managed-notification-people' }) do
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
        render ToggleGroup.new(
          type: :single,
          name: 'notification_preference[managed_person_ids][]',
          value: grant.missed_dose_notifications_enabled? ? managed_person.id.to_s : '',
          variant: :outline,
          size: :sm,
          aria: { label: t('profiles.notifications.managed_people.toggle_label', name: managed_person.name) }
        ) do |group|
          group.toggle_group_item(value: managed_person.id.to_s, class: 'min-h-11') { t('profiles.common.on') }
          group.toggle_group_item(value: '', class: 'min-h-11') { t('profiles.common.off') }
        end
      end
    end
  end
end
