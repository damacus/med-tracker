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
              class: 'text-[10px] font-black uppercase tracking-widest text-slate-400 ml-1'
            ) { t('medications.show.location') }
            render RubyUI::Combobox.new(class: 'w-full') do
              render RubyUI::ComboboxTrigger.new(
                placeholder: selected_location_name || t('forms.medications.select_location'),
                class: field_error_class(medication, :location)
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
              class: 'text-[10px] font-black uppercase tracking-widest text-slate-400 ml-1'
            ) { t('forms.medications.name') }
            render RubyUI::Input.new(
              type: :text,
              name: 'medication[name]',
              id: 'medication_name',
              value: medication.name,
              required: true,
              placeholder: t('forms.medications.name_placeholder'),
              class: 'rounded-md border-slate-200 bg-white py-4 px-4 focus:ring-2 focus:ring-primary/10 ' \
                     "focus:border-primary transition-all #{field_error_class(medication, :name)}"
            )
            render_field_error(medication, :name)
          end
        end

        def render_category_field
          div(class: 'space-y-2') do
            render RubyUI::FormFieldLabel.new(
              for: 'medication_category_trigger',
              class: 'text-[10px] font-black uppercase tracking-widest text-slate-400 ml-1'
            ) { 'Category' }
            render RubyUI::Combobox.new(class: 'w-full') do
              render RubyUI::ComboboxTrigger.new(
                placeholder: medication.category.presence || t('forms.medications.select_category'),
                class: field_error_class(medication, :category)
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
              class: 'text-[10px] font-black uppercase tracking-widest text-slate-400 ml-1'
            ) { t('forms.medications.description') }
            render RubyUI::Textarea.new(
              name: 'medication[description]',
              id: 'medication_description',
              rows: 3,
              placeholder: t('forms.medications.description_placeholder'),
              class: 'rounded-md border-slate-200 bg-white p-4 focus:ring-2 focus:ring-primary/10 ' \
                     'focus:border-primary transition-all resize-none'
            ) { medication.description }
          end
        end

        def render_dosage_amount_field
          div(class: 'space-y-2') do
            render RubyUI::FormFieldLabel.new(
              for: 'medication_dosage_amount',
              class: 'text-[10px] font-black uppercase tracking-widest text-slate-400 ml-1'
            ) { t('forms.medications.standard_dosage') }
            render RubyUI::Input.new(
              type: :number,
              name: 'medication[dosage_amount]',
              id: 'medication_dosage_amount',
              value: medication.dosage_amount.to_i,
              step: 'any',
              min: '1',
              placeholder: t('forms.medications.standard_dosage_placeholder', default: 'e.g., 500'),
              class: 'rounded-md border-slate-200 bg-white py-4 px-4 focus:ring-2 focus:ring-primary/10 ' \
                     'focus:border-primary transition-all'
            )
            render_field_error(medication, :dosage_amount)
          end
        end

        def render_dosage_unit_field
          div(class: 'space-y-2') do
            render RubyUI::FormFieldLabel.new(
              for: 'medication_dosage_unit_trigger',
              class: 'text-[10px] font-black uppercase tracking-widest text-slate-400 ml-1'
            ) { t('forms.medications.unit') }
            render RubyUI::Combobox.new(class: 'w-full') do
              render RubyUI::ComboboxTrigger.new(
                placeholder: medication.dosage_unit.presence || t('forms.medications.select_unit'),
                class: field_error_class(medication, :dosage_unit)
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
              class: 'text-[10px] font-black uppercase tracking-widest text-slate-400 ml-1'
            ) { t('forms.medications.current_supply') }
            render RubyUI::Input.new(
              type: :number,
              name: 'medication[current_supply]',
              id: 'medication_current_supply',
              value: medication.current_supply,
              min: '0',
              placeholder: t('forms.medications.current_supply_placeholder', default: 'e.g., 30'),
              class: 'rounded-md border-slate-200 bg-white py-4 px-4 focus:ring-2 focus:ring-primary/10 ' \
                     'focus:border-primary transition-all'
            )
          end
        end

        def render_reorder_threshold_field
          div(class: 'space-y-2') do
            render RubyUI::FormFieldLabel.new(
              for: 'medication_reorder_threshold',
              class: 'text-[10px] font-black uppercase tracking-widest text-slate-400 ml-1'
            ) { t('forms.medications.reorder_threshold') }
            render RubyUI::Input.new(
              type: :number,
              name: 'medication[reorder_threshold]',
              id: 'medication_reorder_threshold',
              value: medication.reorder_threshold,
              min: '1',
              placeholder: t('forms.medications.reorder_threshold_placeholder', default: 'e.g., 5'),
              class: 'rounded-md border-slate-200 bg-white py-4 px-4 focus:ring-2 focus:ring-primary/10 ' \
                     'focus:border-primary transition-all'
            )
          end
        end

        def render_warnings_field
          div(class: 'space-y-2') do
            render RubyUI::FormFieldLabel.new(
              for: 'medication_warnings',
              class: 'text-[10px] font-black uppercase tracking-widest text-rose-400 ml-1'
            ) { t('forms.medications.warnings') }
            render RubyUI::Textarea.new(
              name: 'medication[warnings]',
              id: 'medication_warnings',
              rows: 3,
              placeholder: t('forms.medications.warnings_placeholder'),
              class: 'rounded-md border-rose-100 bg-rose-50/30 p-4 text-rose-900 focus:ring-2 ' \
                     'focus:ring-rose-500/10 focus:border-rose-500 transition-all resize-none ' \
                     'placeholder:text-rose-400/50'
            ) { medication.warnings }
          end
        end

        def dosage_units
          Medication::DOSAGE_UNITS
        end

        def selected_location_name
          return nil if medication.location_id.blank?

          locations.find { |l| l.id == medication.location_id }&.name
        end
      end
    end
  end
end
