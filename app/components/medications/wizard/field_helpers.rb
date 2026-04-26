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

        def render_primary_dosage_option_fields
          dosage = primary_dosage_record_for_wizard
          index = 0

          div(
            class: 'space-y-4 rounded-3xl border border-outline-variant/50 ' \
                   'bg-surface-container-low p-6 shadow-elevation-1',
            data: { controller: 'frequency-suggestions' }
          ) do
            input(type: 'hidden', name: dosage_field_name(index, 'id'), value: dosage.id) if dosage.persisted?
            hidden_primary_dosage_field(index, 'description', dosage.description)

            div(class: 'grid grid-cols-1 sm:grid-cols-2 gap-4') do
              render_primary_dosage_amount_field(dosage, index)
              render_primary_dosage_unit_field(dosage, index)
            end

            div(class: 'space-y-2') { render_primary_frequency_field(dosage, index) }

            div(class: 'grid grid-cols-1 sm:grid-cols-3 gap-4') do
              render_primary_dosage_default_number_field(
                dosage: dosage,
                index: index,
                config: {
                  field: 'default_max_daily_doses',
                  label: 'Max doses / cycle',
                  value: dosage.default_max_daily_doses,
                  min: '1',
                  target: 'maxDoses'
                }
              )
              render_primary_dosage_default_number_field(
                dosage: dosage,
                index: index,
                config: {
                  field: 'default_min_hours_between_doses',
                  label: 'Min hours apart',
                  value: dosage.default_min_hours_between_doses,
                  min: '0',
                  step: '0.5',
                  target: 'minHours'
                }
              )
              render_primary_dosage_default_cycle_field(dosage, index)
            end

            div(class: 'flex flex-wrap items-center gap-6') do
              render_primary_dosage_default_checkbox(dosage, index, 'default_for_adults', 'Default for adults')
              render_primary_dosage_default_checkbox(
                dosage,
                index,
                'default_for_children',
                'Default for children / dependents'
              )
            end
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
          return if suggested_dosage_records_for_wizard.empty?

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
              suggested_dosage_records_for_wizard.each_with_index do |dosage, index|
                render_suggested_dosage_record(dosage, index + 1)
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

        def primary_dosage_record_for_wizard
          return @primary_dosage_record_for_wizard if defined?(@primary_dosage_record_for_wizard)

          @primary_dosage_record_for_wizard =
            dosage_records_for_wizard.find(&:default_for_adults?) || dosage_records_for_wizard.first
        end

        def suggested_dosage_records_for_wizard
          dosage_records_for_wizard.reject { |dosage| dosage.equal?(primary_dosage_record_for_wizard) }
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

        def render_primary_dosage_amount_field(dosage, index)
          div(class: 'space-y-2') do
            render RubyUI::FormFieldLabel.new(for: "medication_dosage_records_attributes_#{index}_amount") do
              'Amount'
            end
            m3_input(
              type: :number,
              name: dosage_field_name(index, 'amount'),
              id: "medication_dosage_records_attributes_#{index}_amount",
              value: dosage.amount&.to_s,
              step: 'any',
              min: '0',
              required: true,
              **field_error_attributes(
                dosage,
                :amount,
                input_id: "medication_dosage_records_attributes_#{index}_amount"
              )
            )
            render_field_error(dosage, :amount, input_id: "medication_dosage_records_attributes_#{index}_amount")
          end
        end

        def render_primary_dosage_unit_field(dosage, index)
          div(class: 'space-y-2') do
            render RubyUI::FormFieldLabel.new(for: "medication_dosage_records_attributes_#{index}_unit") do
              'Unit'
            end
            select(
              name: dosage_field_name(index, 'unit'),
              id: "medication_dosage_records_attributes_#{index}_unit",
              class: 'flex h-9 w-full rounded-md border border-outline bg-transparent px-3 py-1 text-sm shadow-sm',
              required: true
            ) do
              option(value: '', selected: dosage.unit.blank?) { 'Select unit' }
              dosage_units.each do |unit|
                option(value: unit, selected: dosage.unit == unit) { unit }
              end
            end
          end
        end

        def render_primary_frequency_field(dosage, index)
          render RubyUI::FormFieldLabel.new(for: "medication_dosage_records_attributes_#{index}_frequency") do
            'Frequency label'
          end
          render_frequency_template_buttons
          m3_input(
            type: :text,
            name: dosage_field_name(index, 'frequency'),
            id: "medication_dosage_records_attributes_#{index}_frequency",
            value: dosage.frequency,
            placeholder: 'Every morning',
            required: true,
            data: { 'frequency-suggestions-target': 'input' },
            **field_error_attributes(
              dosage,
              :frequency,
              input_id: "medication_dosage_records_attributes_#{index}_frequency"
            )
          )
          render_field_error(
            dosage,
            :frequency,
            input_id: "medication_dosage_records_attributes_#{index}_frequency"
          )
        end

        def render_primary_dosage_default_number_field(dosage:, index:, config:)
          field = config.fetch(:field)
          div(class: 'space-y-2') do
            render RubyUI::FormFieldLabel.new(for: "medication_dosage_records_attributes_#{index}_#{field}") do
              config.fetch(:label)
            end
            options = {
              type: :number,
              name: dosage_field_name(index, field),
              id: "medication_dosage_records_attributes_#{index}_#{field}",
              value: config[:value]&.to_s,
              min: config.fetch(:min),
              required: true,
              data: { 'frequency-suggestions-target': config.fetch(:target) }
            }
            options[:step] = config[:step] if config[:step]
            options.merge!(
              field_error_attributes(
                dosage,
                field,
                input_id: "medication_dosage_records_attributes_#{index}_#{field}"
              )
            )
            m3_input(**options)
            render_field_error(dosage, field, input_id: "medication_dosage_records_attributes_#{index}_#{field}")
          end
        end

        def render_primary_dosage_default_cycle_field(dosage, index)
          div(class: 'space-y-2') do
            render RubyUI::FormFieldLabel.new(
              for: "medication_dosage_records_attributes_#{index}_default_dose_cycle"
            ) do
              'Dose cycle'
            end
            select(
              name: dosage_field_name(index, 'default_dose_cycle'),
              id: "medication_dosage_records_attributes_#{index}_default_dose_cycle",
              class: 'flex h-9 w-full rounded-md border border-outline bg-transparent px-3 py-1 text-sm shadow-sm',
              required: true,
              data: { 'frequency-suggestions-target': 'doseCycle' }
            ) do
              option(value: '', selected: dosage.default_dose_cycle.blank?) { 'Select cycle' }
              MedicationDosage::DOSE_CYCLE_OPTIONS.each do |label, value|
                option(value: value, selected: dosage.default_dose_cycle == value) { label }
              end
            end
            render_field_error(
              dosage,
              :default_dose_cycle,
              input_id: "medication_dosage_records_attributes_#{index}_default_dose_cycle"
            )
          end
        end

        def render_primary_dosage_default_checkbox(dosage, index, field, label)
          input(type: 'hidden', name: dosage_field_name(index, field), value: '0')
          label(class: 'flex items-center gap-2 text-sm cursor-pointer') do
            input(
              type: 'checkbox',
              name: dosage_field_name(index, field),
              value: '1',
              checked: dosage.public_send("#{field}?"),
              class: 'rounded border-outline'
            )
            span { label }
          end
        end

        def hidden_primary_dosage_field(index, field, value)
          return if value.blank?

          input(type: 'hidden', name: dosage_field_name(index, field), value: serialize_hidden_dosage_value(value))
        end

        def render_frequency_template_buttons
          div(class: 'flex flex-nowrap overflow-x-auto gap-1.5 mt-1 mb-2 pb-0.5 -mx-0.5 px-0.5') do
            Components::Medications::FormView::FREQUENCY_TEMPLATES.each do |template|
              button(
                type: 'button',
                data: {
                  action: 'click->frequency-suggestions#suggest',
                  frequency: template.fetch(:label),
                  'frequency-suggestions-frequency-value': template.fetch(:frequency),
                  'frequency-suggestions-max-doses-value': template.fetch(:max_doses),
                  'frequency-suggestions-min-hours-value': template.fetch(:min_hours),
                  'frequency-suggestions-dose-cycle-value': template.fetch(:dose_cycle)
                },
                class: 'inline-flex shrink-0 items-center rounded-full border border-outline-variant/50 ' \
                       'bg-surface-container px-3 py-1 text-xs font-semibold ' \
                       'text-on-surface-variant shadow-elevation-1 whitespace-nowrap ' \
                       'hover:bg-secondary-container hover:text-on-secondary-container cursor-pointer transition-all'
              ) { template.fetch(:label) }
            end
          end
        end
      end
    end
  end
end
