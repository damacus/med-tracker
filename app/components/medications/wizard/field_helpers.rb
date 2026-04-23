# frozen_string_literal: true

module Components
  module Medications
    module Wizard
      module FieldHelpers # rubocop:disable Metrics/ModuleLength
        extend ActiveSupport::Concern

        private

        def render_location_field
          div(class: 'space-y-2') do
            render RubyUI::FormFieldLabel.new(
              for: 'medication_location_id_trigger',
              class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1'
            ) do
              plain t('medications.show.location')
              span(class: 'text-error ml-0.5') { ' *' }
            end
            render RubyUI::Combobox.new(class: 'w-full') do
              render RubyUI::ComboboxTrigger.new(
                placeholder: selected_location_name || t('forms.medications.select_location'),
                class: "rounded-md #{field_error_class(medication, :location)}"
              )

              render RubyUI::ComboboxPopover.new do
                render RubyUI::ComboboxSearchInput.new(
                  placeholder: t('forms.medications.select_location')
                )

                render RubyUI::ComboboxList.new do
                  render(RubyUI::ComboboxEmptyState.new { t('forms.medications.select_location') })

                  locations.each do |loc|
                    render RubyUI::ComboboxItem.new do
                      render RubyUI::ComboboxRadio.new(
                        name: 'medication[location_id]',
                        id: "medication_location_id_#{loc.id}",
                        value: loc.id,
                        checked: medication.location_id == loc.id,
                        required: true
                      )
                      span { loc.name }
                    end
                  end
                end
              end
            end
            render_field_error(medication, :location)
          end
        end

        def render_name_field
          div(class: 'space-y-2') do
            render RubyUI::FormFieldLabel.new(
              for: 'medication_name',
              class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1'
            ) do
              plain t('forms.medications.name')
              span(class: 'text-error ml-0.5') { ' *' }
            end
            m3_input(
              type: :text,
              name: 'medication[name]',
              id: 'medication_name',
              value: medication.name,
              required: true,
              placeholder: t('forms.medications.name_placeholder'),
              title: 'Medication name, e.g. Ibuprofen',
              class: "rounded-md border-outline-variant bg-surface-container-lowest py-4 px-4 #{field_error_class(
                medication, :name
              )}"
            )
            render_field_error(medication, :name)
          end
        end

        def render_category_field
          div(class: 'space-y-2') do
            render RubyUI::FormFieldLabel.new(
              for: 'medication_category_trigger',
              class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1'
            ) { 'Category' }
            render RubyUI::Combobox.new(class: 'w-full') do
              render RubyUI::ComboboxTrigger.new(
                placeholder: medication.category.presence || t('forms.medications.select_category'),
                class: "rounded-md #{field_error_class(medication, :category)}"
              )

              render RubyUI::ComboboxPopover.new do
                render RubyUI::ComboboxSearchInput.new(
                  placeholder: t('forms.medications.filter_categories')
                )

                render RubyUI::ComboboxList.new do
                  render RubyUI::ComboboxEmptyState.new do
                    t('forms.medications.no_categories_found')
                  end

                  render RubyUI::ComboboxItem.new do
                    render RubyUI::ComboboxRadio.new(
                      name: 'medication[category]',
                      value: '',
                      checked: medication.category.blank?
                    )
                    span { t('forms.medications.select_category') }
                  end

                  Medication::CATEGORIES.each do |category|
                    render RubyUI::ComboboxItem.new do
                      render RubyUI::ComboboxRadio.new(
                        name: 'medication[category]',
                        value: category,
                        checked: medication.category == category
                      )
                      span { category }
                    end
                  end
                end
              end
            end
            render_field_error(medication, :category)
          end
        end

        def render_description_field
          div(class: 'space-y-2') do
            render RubyUI::FormFieldLabel.new(
              for: 'medication_description',
              class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1'
            ) { t('forms.medications.description') }
            render RubyUI::Textarea.new(
              name: 'medication[description]',
              id: 'medication_description',
              rows: 3,
              placeholder: t('forms.medications.description_placeholder'),
              class: 'rounded-md border-outline-variant bg-surface-container-lowest p-4 ' \
                     'focus:ring-2 focus:ring-primary/10 ' \
                     'focus:border-primary transition-all resize-none'
            ) { medication.description }
          end
        end

        def render_dosage_amount_field
          div(class: 'space-y-2') do
            render RubyUI::FormFieldLabel.new(
              for: 'medication_dosage_amount',
              class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1'
            ) { t('forms.medications.standard_dosage') }
            m3_input(
              type: :number,
              name: 'medication[dosage_amount]',
              id: 'medication_dosage_amount',
              value: medication.dosage_amount,
              step: 'any',
              min: '1',
              title: 'Standard dose amount, e.g. 500',
              placeholder: t('forms.medications.standard_dosage_placeholder', default: 'e.g., 500'),
              class: 'rounded-md border-outline-variant bg-surface-container-lowest py-4 px-4 ' \
                     'focus:ring-2 focus:ring-primary/10 ' \
                     'focus:border-primary transition-all'
            )
            render_field_error(medication, :dosage_amount)
          end
        end

        def render_dosage_unit_field
          div(class: 'space-y-2') do
            render RubyUI::FormFieldLabel.new(
              for: 'medication_dosage_unit_trigger',
              class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1'
            ) { t('forms.medications.unit') }
            render RubyUI::Combobox.new(class: 'w-full') do
              render RubyUI::ComboboxTrigger.new(
                placeholder: medication.dosage_unit.presence || t('forms.medications.select_unit'),
                class: "rounded-md #{field_error_class(medication, :dosage_unit)}"
              )

              render RubyUI::ComboboxPopover.new do
                render RubyUI::ComboboxSearchInput.new(
                  placeholder: t('forms.medications.select_unit')
                )

                render RubyUI::ComboboxList.new do
                  render RubyUI::ComboboxEmptyState.new do
                    'No units found.'
                  end

                  render RubyUI::ComboboxItem.new do
                    render RubyUI::ComboboxRadio.new(
                      name: 'medication[dosage_unit]',
                      value: '',
                      checked: medication.dosage_unit.blank?
                    )
                    span { t('forms.medications.select_unit') }
                  end

                  Medication::DOSAGE_UNITS.each do |unit|
                    render RubyUI::ComboboxItem.new do
                      render RubyUI::ComboboxRadio.new(
                        name: 'medication[dosage_unit]',
                        value: unit,
                        checked: medication.dosage_unit == unit
                      )
                      span { unit }
                    end
                  end
                end
              end
            end
            render_field_error(medication, :dosage_unit)
          end
        end

        def render_current_supply_field
          div(class: 'space-y-2') do
            render RubyUI::FormFieldLabel.new(
              for: 'medication_current_supply',
              class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1'
            ) { t('forms.medications.starting_supply', default: 'Starting Supply') }
            m3_input(
              type: :number,
              name: 'medication[current_supply]',
              id: 'medication_current_supply',
              value: medication.current_supply,
              min: '0',
              title: 'Starting supply for a new medication',
              placeholder: t('forms.medications.current_supply_placeholder', default: 'e.g., 30'),
              class: 'rounded-md border-outline-variant bg-surface-container-lowest py-4 px-4 ' \
                     'focus:ring-2 focus:ring-primary/10 ' \
                     'focus:border-primary transition-all'
            )
          end
        end

        def render_reorder_threshold_field
          div(class: 'space-y-2') do
            render RubyUI::FormFieldLabel.new(
              for: 'medication_reorder_threshold',
              class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1'
            ) { t('forms.medications.reorder_threshold') }
            m3_input(
              type: :number,
              name: 'medication[reorder_threshold]',
              id: 'medication_reorder_threshold',
              value: medication.reorder_threshold,
              min: '0',
              title: 'Reorder when supply falls below this level',
              placeholder: t('forms.medications.reorder_threshold_placeholder', default: 'e.g., 5'),
              class: 'rounded-md border-outline-variant bg-surface-container-lowest py-4 px-4 ' \
                     'focus:ring-2 focus:ring-primary/10 ' \
                     'focus:border-primary transition-all'
            )
          end
        end

        def render_warnings_field
          div(class: 'space-y-2') do
            render RubyUI::FormFieldLabel.new(
              for: 'medication_warnings',
              class: 'text-[10px] font-black uppercase tracking-widest text-on-error-container ml-1'
            ) { t('forms.medications.warnings') }
            render RubyUI::Textarea.new(
              name: 'medication[warnings]',
              id: 'medication_warnings',
              rows: 3,
              placeholder: t('forms.medications.warnings_placeholder'),
              class: 'rounded-md border-error/20 bg-error-container/10 p-4 ' \
                     'text-on-error-container focus:ring-2 ' \
                     'focus:ring-error/10 focus:border-error transition-all resize-none ' \
                     'placeholder:text-on-error-container/50 font-medium'
            ) { medication.warnings }
          end
        end

        def dosage_units
          Medication::DOSAGE_UNITS
        end

        def render_suggested_dosage_records_section
          return if dosage_records_for_wizard.empty?

          div(class: 'space-y-4') do
            div(class: 'space-y-1') do
              m3_heading(level: 4, size: '4', class: 'font-bold tracking-tight text-foreground') do
                'Suggested dose options'
              end
              m3_text(size: '2', class: 'text-on-surface-variant') do
                'We found dose guidance for this product and will save it with the medication.'
              end
            end

            div(class: 'space-y-3') do
              dosage_records_for_wizard.each_with_index do |dosage, index|
                render_suggested_dosage_record(dosage, index)
              end
            end
          end
        end

        def selected_location_name
          return nil if medication.location_id.blank?

          locations.find { |l| l.id == medication.location_id }&.name
        end

        def dosage_records_for_wizard
          @dosage_records_for_wizard ||= medication.dosage_records.to_a.sort_by do |dosage|
            [dosage.amount.to_f, dosage.description.to_s, dosage.id || 0]
          end
        end

        def render_suggested_dosage_record(dosage, index)
          div(class: 'rounded-3xl border border-outline-variant/50 bg-surface-container-low p-5 space-y-3') do
            render_hidden_dosage_record_fields(dosage, index)

            div(class: 'flex items-start justify-between gap-4') do
              div(class: 'space-y-1') do
                m3_text(size: '3', class: 'font-bold text-foreground') do
                  dosage_label(dosage)
                end
                if dosage.description.present?
                  m3_text(size: '2', class: 'text-on-surface-variant') { dosage.description }
                end
              end

              span(
                class: 'inline-flex items-center rounded-full bg-primary/10 ' \
                       'px-3 py-1 text-xs font-bold text-primary'
              ) do
                dosage_person_label(dosage)
              end
            end

            m3_text(size: '1', class: 'text-on-surface-variant') do
              dosage_schedule_hint(dosage)
            end
          end
        end

        def render_hidden_dosage_record_fields(dosage, index)
          hidden_dosage_field(index, 'amount', dosage.amount)
          hidden_dosage_field(index, 'unit', dosage.unit)
          hidden_dosage_field(index, 'frequency', dosage.frequency)
          hidden_dosage_field(index, 'description', dosage.description)
          hidden_dosage_field(index, 'default_for_adults', dosage.default_for_adults? ? '1' : '0')
          hidden_dosage_field(index, 'default_for_children', dosage.default_for_children? ? '1' : '0')
          hidden_dosage_field(index, 'default_max_daily_doses', dosage.default_max_daily_doses)
          hidden_dosage_field(index, 'default_min_hours_between_doses', dosage.default_min_hours_between_doses)
          hidden_dosage_field(index, 'default_dose_cycle', dosage.default_dose_cycle)
          hidden_dosage_field(index, 'current_supply', dosage.current_supply)
          hidden_dosage_field(index, 'reorder_threshold', dosage.reorder_threshold)
        end

        def hidden_dosage_field(index, field, value)
          input(type: 'hidden', name: dosage_field_name(index, field), value: serialize_hidden_dosage_value(value))
        end

        def dosage_field_name(index, field)
          "medication[dosage_records_attributes][#{index}][#{field}]"
        end

        def serialize_hidden_dosage_value(value)
          return '' if value.nil?
          return value.to_s('F') if value.is_a?(BigDecimal)

          value.to_s
        end

        def dosage_label(dosage)
          [
            DoseAmount.new(dosage.amount, dosage.unit).to_s,
            dosage.frequency.presence
          ].compact.join(' • ')
        end

        def dosage_person_label(dosage)
          return 'Children' if dosage.default_for_children?
          return 'Adults' if dosage.default_for_adults?

          'Suggested'
        end

        def dosage_schedule_hint(dosage)
          parts = []
          if dosage.default_min_hours_between_doses.present?
            parts << "Min #{dosage.default_min_hours_between_doses} hours apart"
          end
          if dosage.default_max_daily_doses.present? && dosage.default_dose_cycle.present?
            parts << "Max #{dosage.default_max_daily_doses} doses per #{dosage.default_dose_cycle}"
          end
          parts.join(' • ')
        end
      end
    end
  end
end
