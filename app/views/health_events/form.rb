# frozen_string_literal: true

module Views
  module HealthEvents
    class Form < Views::Base
      include Phlex::Rails::Helpers::FormWith

      def initialize(person:, health_event:, medication_options:, selected_medication_ids:)
        @person = person
        @health_event = health_event
        @medication_options = medication_options
        @selected_medication_ids = selected_medication_ids
        super()
      end

      def view_template
        div(class: 'container mx-auto max-w-3xl space-y-8 px-4 py-12') do
          render_header
          render_form
        end
      end

      private

      attr_reader :person, :health_event, :medication_options, :selected_medication_ids

      def render_header
        div(class: 'space-y-2') do
          m3_heading(level: 1, size: '7', class: 'font-black') { form_title }
          m3_text(size: '2', class: 'text-on-surface-variant') { t('health_events.form.subtitle') }
        end
      end

      def render_form
        m3_card(class: 'border border-border/70 bg-card p-6') do
          form_with(model: [person, health_event], class: 'space-y-6') do |form|
            render_errors
            render_form_fields(form)
            render_actions
          end
        end
      end

      def render_form_fields(form)
        input(type: 'hidden', name: 'health_event[event_kind]', value: health_event.event_kind)
        render_field(form, :title, t('health_events.form.title'))
        render_date_fields(form)
        render_severity_field(form)
        render_medication_checkboxes if health_event.suspected_side_effect?
        render_textarea(form, :notes, t('health_events.form.notes'))
        render_textarea(form, :action_taken, t('health_events.form.action_taken'))
        render_medical_help_field(form)
      end

      def render_errors
        return if health_event.errors.empty?

        div(class: 'rounded-xl border border-destructive/40 bg-destructive/10 p-4 text-sm text-destructive') do
          ul { health_event.errors.full_messages.each { |message| li { message } } }
        end
      end

      def render_field(form, field, label)
        div(class: 'space-y-2') do
          form.label(field, label, class: 'text-sm font-bold')
          form.text_field(field, class: input_class)
        end
      end

      def render_date_fields(form)
        div(class: 'grid gap-4 sm:grid-cols-2') do
          div(class: 'space-y-2') do
            form.label(:started_on, t('health_events.form.started_on'), class: 'text-sm font-bold')
            form.date_field(:started_on, class: input_class)
          end
          div(class: 'space-y-2') do
            form.label(:ended_on, t('health_events.form.ended_on'), class: 'text-sm font-bold')
            form.date_field(:ended_on, class: input_class)
            label(class: 'flex items-center gap-2 text-sm') do
              input(type: 'checkbox', name: 'health_event[ongoing]', value: '1', checked: health_event.ongoing?)
              plain t('health_events.form.ongoing')
            end
          end
        end
      end

      def render_severity_field(form)
        div(class: 'space-y-2') do
          form.label(:severity, t('health_events.form.severity'), class: 'text-sm font-bold')
          form.select(
            :severity,
            [[t('health_events.form.no_severity'), '']] + HealthEvent.severities.keys.map do |severity|
              [t("health_events.severities.#{severity}"), severity]
            end,
            {},
            class: input_class
          )
        end
      end

      def render_medication_checkboxes
        div(class: 'space-y-3') do
          m3_text(size: '2', class: 'font-bold') { t('health_events.form.suspected_medications') }
          medication_options.each do |medication|
            label(class: 'flex items-center gap-2 text-sm') do
              input(type: 'checkbox', name: 'medication_ids[]', value: medication.id,
                    checked: selected_medication_ids.include?(medication.id))
              plain medication.name
            end
          end
        end
      end

      def render_textarea(form, field, label)
        div(class: 'space-y-2') do
          form.label(field, label, class: 'text-sm font-bold')
          form.text_area(field, rows: 4, class: input_class)
        end
      end

      def render_medical_help_field(form)
        label(class: 'flex items-center gap-2 text-sm') do
          form.check_box(:medical_help_sought)
          plain t('health_events.form.medical_help_sought')
        end
      end

      def render_actions
        div(class: 'flex items-center justify-end gap-3') do
          m3_link(href: person_health_events_path(person), variant: :text) { t('health_events.actions.cancel') }
          m3_button(type: :submit, variant: :filled) { t('health_events.actions.save') }
        end
      end

      def form_title
        key = health_event.persisted? ? 'edit_title' : 'new_title'
        t("health_events.form.#{key}", kind: t("health_events.kinds.#{health_event.event_kind}").downcase)
      end

      def input_class
        'w-full rounded-xl border border-input bg-background px-3 py-2 text-sm'
      end
    end
  end
end
