# frozen_string_literal: true

module Views
  module HealthEvents
    class Index < Views::Base
      include Phlex::Rails::Helpers::L

      def initialize(person:, health_events:)
        @person = person
        @health_events = health_events
        super()
      end

      def view_template
        div(class: 'container mx-auto max-w-5xl space-y-8 px-4 py-12') do
          render_header
          render_events
        end
      end

      private

      attr_reader :person, :health_events

      def render_header
        div(class: 'flex flex-col gap-4 border-b border-outline-variant/30 pb-6 md:flex-row md:items-end md:justify-between') do
          div(class: 'space-y-2') do
            m3_heading(level: 1, size: '7', class: 'font-black') { t('health_events.index.title', person: person.name) }
            m3_text(size: '2', class: 'text-on-surface-variant') { t('health_events.index.subtitle') }
          end
          div(class: 'flex flex-col gap-3 sm:flex-row') do
            new_event_link(:illness, t('health_events.actions.record_illness'))
            new_event_link(:suspected_side_effect, t('health_events.actions.record_suspected_side_effect'))
          end
        end
      end

      def new_event_link(event_kind, label)
        m3_link(
          href: new_person_health_event_path(person, event_kind: event_kind),
          variant: event_kind == :illness ? :outlined : :filled,
          size: :lg,
          class: 'font-bold'
        ) { label }
      end

      def render_events
        if health_events.any?
          div(class: 'space-y-4') { health_events.each { |event| render_event(event) } }
        else
          m3_card(class: 'border border-dashed border-outline-variant p-8 text-center') do
            m3_text(size: '2', class: 'text-on-surface-variant') { t('health_events.index.empty') }
          end
        end
      end

      def render_event(event)
        m3_card(class: 'space-y-4 border border-border/70 bg-card p-6') do
          div(class: 'flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between') do
            div(class: 'space-y-2') do
              m3_badge(variant: :outlined, class: 'w-fit') { t("health_events.kinds.#{event.event_kind}") }
              m3_heading(level: 2, size: '4', class: 'font-bold') { event.title }
              m3_text(size: '2', class: 'text-on-surface-variant') { event_date_range(event) }
              render_event_details(event)
            end
            render_event_actions(event)
          end
        end
      end

      def render_event_details(event)
        details = event_details(event)
        return if details.empty?

        m3_text(size: '2', class: 'text-on-surface') { details.join(' · ') }
      end

      def event_details(event)
        [
          severity_detail(event),
          medication_detail(event),
          event.notes.presence,
          event.action_taken.presence
        ].compact
      end

      def severity_detail(event)
        t("health_events.severities.#{event.severity}") if event.severity.present?
      end

      def medication_detail(event)
        return unless event.health_event_medications.any?

        event.health_event_medications.map(&:medication_name).to_sentence
      end

      def render_event_actions(event)
        div(class: 'flex gap-2') do
          m3_link(href: edit_person_health_event_path(person, event), variant: :text, size: :sm) do
            t('health_events.actions.edit')
          end
          form(action: person_health_event_path(person, event), method: 'post') do
            input(type: 'hidden', name: '_method', value: 'delete')
            input(type: 'hidden', name: 'authenticity_token', value: view_context.form_authenticity_token)
            button(type: 'submit', class: 'text-sm font-bold text-destructive') { t('health_events.actions.delete') }
          end
        end
      end

      def event_date_range(event)
        return t('health_events.index.ongoing_from', started_on: l(event.started_on)) if event.ongoing?

        t('health_events.index.date_range', started_on: l(event.started_on), ended_on: l(event.ended_on))
      end
    end
  end
end
