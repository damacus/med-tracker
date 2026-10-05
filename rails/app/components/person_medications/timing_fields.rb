# frozen_string_literal: true

module Components
  module PersonMedications
    class TimingFields < Components::Base
      attr_reader :person_medication

      def initialize(person_medication:)
        @person_medication = person_medication
        super()
      end

      def view_template
        div(class: 'grid min-w-0 grid-cols-1 gap-4 md:grid-cols-3') do
          render_max_daily_doses_field
          render_min_hours_field
          render_dose_cycle_field
        end
      end

      private

      def render_max_daily_doses_field
        FormField do
          FormFieldLabel(for: 'person_medication_max_daily_doses') do
            t('person_medications.form.max_doses_per_cycle')
          end
          m3_input(
            type: :number,
            name: 'person_medication[max_daily_doses]',
            id: 'person_medication_max_daily_doses',
            value: person_medication.max_daily_doses,
            min: 1,
            placeholder: t('person_medications.form.optional'),
            data: { person_medication_form_target: 'maxDosesInput' }
          )
          FormFieldHint { t('person_medications.form.max_doses_hint') }
        end
      end

      def render_min_hours_field
        FormField do
          FormFieldLabel(for: 'person_medication_min_hours_between_doses') do
            t('person_medications.form.min_hours_apart')
          end
          m3_input(
            type: :number,
            name: 'person_medication[min_hours_between_doses]',
            id: 'person_medication_min_hours_between_doses',
            value: person_medication.min_hours_between_doses,
            min: 0,
            step: 0.5,
            placeholder: t('person_medications.form.optional'),
            data: { person_medication_form_target: 'minHoursInput' }
          )
          FormFieldHint { t('person_medications.form.min_hours_hint') }
        end
      end

      def render_dose_cycle_field
        FormField do
          FormFieldLabel(for: 'person_medication_dose_cycle_trigger') { t('person_medications.form.dose_cycle') }
          render RubyUI::Combobox.new(class: 'w-full min-w-0') do
            render RubyUI::ComboboxTrigger.new(
              placeholder: person_medication.dose_cycle&.titleize || t('person_medications.form.default_dose_cycle')
            )

            render RubyUI::ComboboxPopover.new do
              render RubyUI::ComboboxList.new do
                render(RubyUI::ComboboxEmptyState.new { t('person_medications.form.no_options') })

                PersonMedication::DOSE_CYCLE_OPTIONS.each do |label, value|
                  render RubyUI::ComboboxItem.new do
                    render RubyUI::ComboboxRadio.new(
                      name: 'person_medication[dose_cycle]',
                      id: "person_medication_dose_cycle_#{value}",
                      value: value,
                      checked: person_medication.dose_cycle == value,
                      data: { person_medication_form_target: 'doseCycleInput' }
                    )
                    span { label }
                  end
                end
              end
            end
          end
          FormFieldHint { t('person_medications.form.dose_cycle_hint') }
        end
      end
    end
  end
end
