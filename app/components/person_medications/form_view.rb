# frozen_string_literal: true

module Components
  module PersonMedications
    # Form view for adding or editing a person medication (OTC/supplement)
    class FormView < Components::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::OptionsFromCollectionForSelect
      include Phlex::Rails::Helpers::Pluralize

      attr_reader :person_medication, :person, :medications, :editing

      def initialize(person_medication:, person:, medications:, editing: false)
        @person_medication = person_medication
        @person = person
        @medications = medications
        @editing = editing
        super()
      end

      def view_template
        div(class: 'container mx-auto px-4 py-8 max-w-2xl') do
          render_header
          render_form
        end
      end

      private

      def render_header
        div(class: 'mb-8') do
          m3_text(size: '2', weight: 'medium', class: 'uppercase tracking-wide text-on-surface-variant mb-2') do
            t('person_medications.form.add_medication')
          end
          m3_heading(level: 1) do
            if editing
              t('person_medications.form.edit_medication_for', person: person.name)
            else
              t('person_medications.form.add_medication_for', person: person.name)
            end
          end
        end
      end

      def render_form
        form_with(
          model: person_medication,
          url: if editing
                 person_person_medication_path(person,
                                               person_medication)
               else
                 person_person_medications_path(person)
               end,
          method: editing ? :patch : :post,
          class: 'space-y-6',
          data: {
            controller: 'person-medication-form',
            person_type: person.person_type,
            person_medication_form_dose_options_value: medication_dose_options.to_json
          }
        ) do |form|
          render_errors if person_medication.errors.any?
          render_form_fields(form)
          render_actions
        end
      end

      def render_errors
        Alert(variant: :destructive, class: 'mb-6') do
          AlertTitle do
            t('person_medications.form.validation_errors', count: person_medication.errors.count)
          end
          AlertDescription do
            ul(class: 'my-2 ml-6 list-disc [&>li]:mt-1') do
              person_medication.errors.full_messages.each do |message|
                li { message }
              end
            end
          end
        end
      end

      def render_form_fields(_form)
        render FormFields.new(person_medication: person_medication, medications: medications, editing: editing)
      end

      def render_actions
        div(class: 'flex justify-end gap-3 pt-4') do
          Link(href: person_path(person), variant: :outlined) { t('person_medications.form.cancel') }
          m3_button(type: :submit, variant: :filled) do
            if editing
              t('person_medications.form.save_changes_button')
            else
              t('person_medications.form.add_medication_button')
            end
          end
        end
      end

      def medication_dose_options
        medications.each_with_object({}) do |medication, dose_options|
          dose_options[medication.id.to_s] = medication.dose_options_payload
        end
      end
    end
  end
end