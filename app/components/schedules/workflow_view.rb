# frozen_string_literal: true

module Components
  module Schedules
    class WorkflowView < Components::Base
      include Phlex::Rails::Helpers::FormWith

      attr_reader :people, :medications, :selected_person_id, :selected_medication_id, :schedule_type, :frequency

      def initialize(people:, medications:, **selection)
        @people = people
        @medications = medications
        @selected_person_id = selection[:selected_person_id]
        @selected_medication_id = selection[:selected_medication_id]
        @schedule_type = selection[:schedule_type]
        @frequency = selection[:frequency]
        super()
      end

      def view_template
        div(class: 'container mx-auto px-4 py-8 max-w-4xl space-y-8', data: { testid: 'schedule-workflow' }) do
          render_header
          render_form
          render_summary
        end
      end

      private

      def render_header
        div(class: 'space-y-2 mb-10') do
          m3_text(variant: :label_medium, class: 'uppercase tracking-[0.2em] font-black opacity-40') do
            t('schedules.workflow.eyebrow')
          end
          m3_heading(variant: :display_small, level: 1, class: 'font-black tracking-tight text-foreground') do
            t('schedules.workflow.title')
          end
        end
      end

      def render_form
        form_with(url: start_schedules_workflow_path, method: :post, class: 'space-y-6') do
          div(class: 'grid grid-cols-1 md:grid-cols-2 gap-6') do
            render_schedule_type_field
            render_person_field
            render_medication_field
            render_frequency_field
          end

          div(class: 'flex justify-end') do
            m3_button(type: :submit, variant: :filled) { t('schedules.workflow.continue_button') }
          end
        end
      end

      def render_schedule_type_field
        FormField do
          render RubyUI::FormFieldLabel.new(for: 'schedule_type', class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1') { t('schedules.workflow.schedule_type_label') }
          select(name: 'schedule_type', id: 'schedule_type',
                 class: 'w-full rounded-md border border-outline bg-surface-container-lowest px-3 py-4 text-sm') do
            option(value: '', selected: schedule_type.blank?) { t('schedules.workflow.schedule_type_placeholder') }
            option(value: 'otc', selected: schedule_type == 'otc') { t('schedules.workflow.schedule_type_options.otc') }
            option(value: 'prescribed', selected: schedule_type == 'prescribed') do
              t('schedules.workflow.schedule_type_options.prescribed')
            end
          end
        end
      end

      def render_person_field
        FormField do
          render RubyUI::FormFieldLabel.new(for: 'person_id', class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1') { t('schedules.workflow.person_label') }
          select(name: 'person_id', id: 'person_id', required: true,
                 class: 'w-full rounded-md border border-outline bg-surface-container-lowest px-3 py-4 text-sm') do
            option(value: '') { t('schedules.workflow.person_placeholder') }
            people.each do |person|
              option(value: person.id, selected: person.id.to_s == selected_person_id.to_s) { person.name }
            end
          end
        end
      end

      def render_medication_field
        FormField do
          render RubyUI::FormFieldLabel.new(for: 'medication_id', class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1') { t('schedules.workflow.medication_label') }
          select(name: 'medication_id', id: 'medication_id', required: true,
                 class: 'w-full rounded-md border border-outline bg-surface-container-lowest px-3 py-4 text-sm') do
            option(value: '') { t('schedules.workflow.medication_placeholder') }
            medications.each do |medication|
              option(value: medication.id, selected: medication.id.to_s == selected_medication_id.to_s) do
                medication.name
              end
            end
          end
        end
      end

      def render_frequency_field
        FormField do
          render RubyUI::FormFieldLabel.new(for: 'frequency', class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1') { t('schedules.workflow.frequency_label') }
          m3_input(
            type: :text,
            name: 'frequency',
            id: 'frequency',
            value: frequency,
            placeholder: t('schedules.workflow.frequency_placeholder'),
            class: 'rounded-md border-outline-variant bg-surface-container-lowest py-4 px-4 transition-all'
          )
        end
      end

      def render_summary
        m3_card(variant: :elevated, class: 'p-8 border-none shadow-elevation-2 rounded-[2.5rem]') do
          m3_heading(variant: :title_large, level: 2, class: 'font-bold mb-6 tracking-tight') { t('schedules.workflow.summary_title') }
          div(class: 'grid grid-cols-1 md:grid-cols-2 gap-6') do
            render_summary_row(t('schedules.workflow.medication_label'), selected_medication&.name)
            render_summary_row(t('schedules.workflow.person_label'), selected_person&.name)
            render_summary_row(t('schedules.workflow.schedule_type_label'), selected_schedule_type)
            render_summary_row(t('schedules.workflow.frequency_label'), frequency)
          end
        end
      end

      def render_summary_row(label, value)
        div(class: 'space-y-1') do
          m3_text(variant: :label_small, class: 'uppercase tracking-widest text-on-surface-variant font-black') { label }
          m3_text(variant: :body_large, class: 'font-bold text-foreground') { summary_value(value) }
        end
      end

      def summary_value(value)
        value.presence || t('schedules.workflow.none')
      end

      def selected_schedule_type
        return t('schedules.workflow.schedule_type_options.otc') if schedule_type == 'otc'
        return t('schedules.workflow.schedule_type_options.prescribed') if schedule_type == 'prescribed'

        nil
      end

      def selected_medication
        medications.find { |medication| medication.id.to_s == selected_medication_id.to_s }
      end

      def selected_person
        people.find { |person| person.id.to_s == selected_person_id.to_s }
      end
    end
  end
end