# frozen_string_literal: true

module Components
  module Schedules
    class WorkflowView < Components::Base
      include Phlex::Rails::Helpers::FormWith

      attr_reader :people, :medications, :selected_person_id, :selected_medication_id, :schedule_type, :frequency

      def initialize(people:, medications:, selected_person_id: nil, selected_medication_id: nil, schedule_type: nil, frequency: nil)
        @people = people
        @medications = medications
        @selected_person_id = selected_person_id
        @selected_medication_id = selected_medication_id
        @schedule_type = schedule_type
        @frequency = frequency
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
        div(class: 'space-y-2') do
          Text(size: '2', weight: 'medium', class: 'uppercase tracking-wide text-slate-500') { 'Schedule workflow' }
          Heading(level: 1) { 'Create a medication schedule' }
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
            Button(type: :submit, variant: :primary) { 'Continue to schedule details' }
          end
        end
      end

      def render_schedule_type_field
        FormField do
          FormFieldLabel(for: 'schedule_type') { 'Type (OTC or prescribed)' }
          select(name: 'schedule_type', id: 'schedule_type', class: 'w-full rounded-md border border-input bg-background px-3 py-2 text-sm') do
            option(value: '', selected: schedule_type.blank?) { 'Select type' }
            option(value: 'otc', selected: schedule_type == 'otc') { 'OTC' }
            option(value: 'prescribed', selected: schedule_type == 'prescribed') { 'Prescribed' }
          end
        end
      end

      def render_person_field
        FormField do
          FormFieldLabel(for: 'person_id') { 'Person name' }
          select(name: 'person_id', id: 'person_id', required: true,
                 class: 'w-full rounded-md border border-input bg-background px-3 py-2 text-sm') do
            option(value: '') { 'Select person' }
            people.each do |person|
              option(value: person.id, selected: person.id.to_s == selected_person_id.to_s) { person.name }
            end
          end
        end
      end

      def render_medication_field
        FormField do
          FormFieldLabel(for: 'medication_id') { 'Name of med' }
          select(name: 'medication_id', id: 'medication_id', required: true,
                 class: 'w-full rounded-md border border-input bg-background px-3 py-2 text-sm') do
            option(value: '') { 'Select medication' }
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
          FormFieldLabel(for: 'frequency') { 'Dose, frequency' }
          Input(
            type: :text,
            name: 'frequency',
            id: 'frequency',
            value: frequency,
            placeholder: 'e.g. Once daily'
          )
        end
      end

      def render_summary
        selected_medication = medications.find { |medication| medication.id.to_s == selected_medication_id.to_s }
        selected_person = people.find { |person| person.id.to_s == selected_person_id.to_s }

        render RubyUI::Card.new(class: 'p-6') do
          Heading(level: 2, size: '4', class: 'font-semibold mb-4') { 'Schedule (break this down)' }
          div(class: 'space-y-2 text-sm text-slate-600') do
            p { "Name of med: #{selected_medication&.name || '-'}" }
            p { "Person name: #{selected_person&.name || '-'}" }
            p { "Type: #{schedule_type.presence || '-'}" }
            p { "Dose, frequency: #{frequency.presence || '-'}" }
          end
        end
      end
    end
  end
end
