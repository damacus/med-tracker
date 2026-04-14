# frozen_string_literal: true

module Components
  module Medications
    class FormView < Components::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::Pluralize
      include Wizard::FieldHelpers

      FREQUENCY_TEMPLATES = [
        { label: 'Every morning', frequency: 'Every morning', max_doses: '1', min_hours: '24', dose_cycle: 'daily' },
        { label: 'Every evening', frequency: 'Every evening', max_doses: '1', min_hours: '24', dose_cycle: 'daily' },
        { label: 'Twice daily', frequency: 'Twice daily', max_doses: '2', min_hours: '12', dose_cycle: 'daily' },
        {
          label: 'Three times daily',
          frequency: 'Three times daily',
          max_doses: '3',
          min_hours: '8',
          dose_cycle: 'daily'
        },
        { label: 'Every 4 hours', frequency: 'Every 4 hours', max_doses: '6', min_hours: '4', dose_cycle: 'daily' },
        { label: 'Every 4-6 hours', frequency: 'Every 4-6 hours', max_doses: '6', min_hours: '4', dose_cycle: 'daily' },
        { label: 'Once weekly', frequency: 'Once weekly', max_doses: '1', min_hours: '168', dose_cycle: 'weekly' }
      ].freeze

      attr_reader :medication, :title, :subtitle, :locations, :return_to

      def initialize(medication:, title:, subtitle: nil, locations: [], return_to: nil)
        @medication = medication
        @title = title
        @subtitle = subtitle
        @locations = locations
        @return_to = return_to
        super()
      end

      def view_template
        div(class: 'container mx-auto px-4 py-12 max-w-2xl') do
          render_header
          render_form
        end
      end

      private

      def render_header
        div(class: 'text-center mb-10 space-y-2') do
          div(
            class: 'mx-auto w-16 h-16 rounded-[1.5rem] bg-primary/10 flex items-center justify-center ' \
                   'text-primary shadow-inner mb-6'
          ) do
            render Icons::Pill.new(size: 32)
          end
          Text(size: '2', weight: 'bold', class: 'uppercase tracking-[0.2em] font-black opacity-40') do
            t('forms.medications.inventory_management')
          end
          Heading(level: 1, size: '8', class: 'font-black tracking-tight text-foreground') { title }
          Text(size: '3', class: 'text-muted-foreground') { subtitle } if subtitle
        end
      end

      def render_form
        form_with(
          model: medication,
          class: 'space-y-8',
          data: { testid: 'medication-form' }
        ) do |form|
          render_errors(form) if medication.errors.any?
          input(type: 'hidden', name: 'return_to', value: return_to) if return_to.present?

          Card(class: 'overflow-visible border-none shadow-2xl rounded-[2.5rem] bg-card') do
            div(class: 'p-10 space-y-8') do
              div(class: 'space-y-6') do
                render_location_field(form)
                render_name_field(form)
                render_category_field(form)
                render_description_field(form)
              end

              div(class: 'h-px bg-surface-container w-full')

              div(class: 'space-y-6') do
                Heading(level: 3, size: '4', class: 'font-bold text-foreground') do
                  t('forms.medications.dosage_and_supply')
                end
                div(class: 'grid grid-cols-1 sm:grid-cols-2 gap-6') do
                  render_dosage_fields(form)
                  render_supply_fields(form)
                end
              end

              div(class: 'h-px bg-surface-container w-full')

              render_dosage_options_section

              div(class: 'h-px bg-surface-container w-full')

              render_warnings_field(form)
            end

            div(
              class: 'px-10 py-6 bg-card border-t border-border ' \
                     'flex items-center justify-between gap-4'
            ) do
              Link(href: return_to.presence || medications_path, variant: :ghost,
                   class: 'font-bold text-muted-foreground hover:text-foreground') do
                t('forms.medications.back')
              end
              Button(type: :submit, variant: :primary, size: :lg,
                     class: 'px-8 rounded-shape-xl shadow-lg shadow-primary/20') do
                t('forms.medications.save_medication')
              end
            end
          end
        end
      end

      def render_errors(_form)
        render RubyUI::Alert.new(variant: :destructive, class: 'mb-8 rounded-shape-xl border-none shadow-sm') do
          div(class: 'flex items-start gap-3') do
            render Icons::AlertCircle.new(size: 20)
            div do
              Heading(level: 2, size: '3', class: 'font-bold mb-1') do
                plain t('forms.medications.validation_errors', count: medication.errors.count)
              end
              ul(class: 'text-sm opacity-90 list-disc pl-4 space-y-1') do
                medication.errors.full_messages.each do |message|
                  li { message }
                end
              end
            end
          end
        end
      end

      # Form fields renderers remain mostly the same logic, but we can enhance the
      # input styles if needed via global classes.
      # For now, relying on the central input styling updates we made earlier.

      def render_form_fields(form)
        # This method is now redundant as logic is moved to render_form,
        # but keeping helper methods below for field rendering
      end

      def render_location_field(_form)
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(
            for: 'medication_location_id_trigger',
            class: 'text-[10px] font-black uppercase tracking-widest text-muted-foreground ml-1'
          ) do
            plain t('medications.show.location')
            span(class: 'text-destructive ml-0.5') { ' *' }
          end
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

      def render_name_field(_form)
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(
            for: 'medication_name',
            class: 'text-[10px] font-black uppercase tracking-widest text-muted-foreground ml-1'
          ) do
            plain t('forms.medications.name')
            span(class: 'text-destructive ml-0.5') { ' *' }
          end
          render RubyUI::Input.new(
            type: :text,
            name: 'medication[name]',
            id: 'medication_name',
            value: medication.name,
            required: true,
            placeholder: t('forms.medications.name_placeholder'),
            class: 'rounded-md border-border bg-card py-4 px-4 ' \
                   'focus:ring-2 focus:ring-primary/10 ' \
                   "focus:border-primary transition-all #{field_error_class(medication, :name)}",
            **field_error_attributes(medication, :name, input_id: 'medication_name')
          )
          render_field_error(medication, :name, input_id: 'medication_name')
        end
      end

      def render_category_field(_form)
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(
            for: 'medication_category_trigger',
            class: 'text-[10px] font-black uppercase tracking-widest text-muted-foreground ml-1'
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

      def render_description_field(_form)
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(
            for: 'medication_description',
            class: 'text-[10px] font-black uppercase tracking-widest text-muted-foreground ml-1'
          ) { t('forms.medications.description') }
          render RubyUI::Textarea.new(
            name: 'medication[description]',
            id: 'medication_description',
            rows: 3,
            placeholder: t('forms.medications.description_placeholder'),
            class: 'rounded-md border-border bg-card p-4 focus:ring-2 focus:ring-primary/10 ' \
                   'focus:border-primary transition-all resize-none'
          ) { medication.description }
        end
      end

      def render_dosage_fields(_form)
        render_dosage_amount_field
        render_dosage_unit_field
      end

      def render_dosage_amount_field
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(
            for: 'medication_dosage_amount',
            class: 'text-[10px] font-black uppercase tracking-widest text-muted-foreground ml-1'
          ) { t('forms.medications.standard_dosage') }
          render RubyUI::Input.new(
            type: :number,
            name: 'medication[dosage_amount]',
            id: 'medication_dosage_amount',
            value: medication.dosage_amount.to_i,
            step: 'any',
            min: '1',
            placeholder: t('forms.medications.standard_dosage_placeholder', default: 'e.g., 500'),
            class: 'rounded-md border-border bg-card py-4 px-4 ' \
                   'focus:ring-2 focus:ring-primary/10 ' \
                   'focus:border-primary transition-all',
            **field_error_attributes(medication, :dosage_amount, input_id: 'medication_dosage_amount')
          )
          render_field_error(medication, :dosage_amount, input_id: 'medication_dosage_amount')
        end
      end

      def render_dosage_unit_field
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(
            for: 'medication_dosage_unit_trigger',
            class: 'text-[10px] font-black uppercase tracking-widest text-muted-foreground ml-1'
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

                dosage_units.each do |unit|
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

      def dosage_units
        Medication::DOSAGE_UNITS
      end

      def selected_location_name
        return nil if medication.location_id.blank?

        locations.find { |l| l.id == medication.location_id }&.name
      end

      def render_supply_fields(_form)
        render_current_supply_field
        render_reorder_threshold_field
      end

      def render_dosage_options_section
        div(class: 'space-y-6', data: { controller: 'dosage-options' }) do
          Heading(level: 3, size: '4', class: 'font-bold text-foreground') { 'Dose Options' }
          Text(size: '2', class: 'text-muted-foreground') do
            'Manage all medication-owned dose options here. Schedules copy these values when they are created.'
          end
          div(class: 'space-y-6') do
            dosage_form_rows.each_with_index do |dosage, index|
              render_dosage_option_fields(dosage, index)
            end
          end
        end
      end

      def dosage_form_rows
        @dosage_form_rows ||= begin
          rows = medication.dosage_records.to_a.sort_by { |dosage| [dosage.amount.to_f, dosage.id || 0] }
          rows << medication.dosage_records.build if rows.none?(&:new_record?)
          rows
        end
      end

      def render_dosage_option_fields(dosage, index)
        div(
          class: 'rounded-3xl border border-border/70 bg-white p-6 shadow-sm space-y-4',
          data: { 'dosage-options-target': 'option' }
        ) do
          input(type: :hidden, name: dosage_field_name(index, 'id'), value: dosage.id) if dosage.persisted?
          input(
            type: :hidden,
            name: dosage_field_name(index, '_destroy'),
            value: '0',
            data: { 'dosage-options-target': 'destroyField' }
          )

          div(
            class: 'space-y-4',
            data: {
              'dosage-options-target': 'editor',
              controller: 'frequency-suggestions'
            }
          ) do
            div(class: 'grid grid-cols-1 sm:grid-cols-2 gap-4') do
              render_dosage_option_amount_field(dosage, index)
              render_dosage_option_unit_field(dosage, index)
            end

            div(class: 'space-y-2') { render_frequency_field(dosage, index) }

            div(class: 'space-y-2') do
              render RubyUI::FormFieldLabel.new(for: "medication_dosage_records_attributes_#{index}_description") do
                'Description / notes'
              end
              render RubyUI::Input.new(
                type: :text,
                name: dosage_field_name(index, 'description'),
                id: "medication_dosage_records_attributes_#{index}_description",
                value: dosage.description,
                placeholder: 'Optional'
              )
            end

            div(class: 'grid grid-cols-1 sm:grid-cols-3 gap-4') do
              render_dosage_default_number_field(
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
              render_dosage_default_number_field(
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
              render_dosage_default_cycle_field(dosage, index)
            end

            div(class: 'flex flex-wrap items-center gap-6') do
              render_dosage_default_checkbox(dosage, index, 'default_for_adults', 'Default for adults')
              render_dosage_default_checkbox(dosage, index, 'default_for_children', 'Default for children / dependents')
              render_remove_dosage_option_button
            end
          end

          render_removed_dosage_option_state
        end
      end

      def render_frequency_field(dosage, index)
        render RubyUI::FormFieldLabel.new(for: "medication_dosage_records_attributes_#{index}_frequency") do
          'Frequency label'
        end
        render_frequency_template_buttons
        render RubyUI::Input.new(
          type: :text,
          name: dosage_field_name(index, 'frequency'),
          id: "medication_dosage_records_attributes_#{index}_frequency",
          value: dosage.frequency,
          placeholder: 'Every morning',
          required: dosage_row_requires_input?(dosage),
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

      def render_frequency_template_buttons
        div(class: 'flex flex-nowrap overflow-x-auto gap-1.5 mt-1 mb-2 pb-0.5 -mx-0.5 px-0.5') do
          FREQUENCY_TEMPLATES.each do |template|
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
              class: 'inline-flex shrink-0 items-center rounded-full border border-border/80 ' \
                     'bg-white px-3 py-1 text-xs font-semibold text-foreground/80 shadow-sm ' \
                     'whitespace-nowrap hover:bg-primary/5 hover:border-primary/30 ' \
                     'cursor-pointer transition-colors'
            ) { template.fetch(:label) }
          end
        end
      end

      def render_remove_dosage_option_button
        button(
          type: 'button',
          class: 'inline-flex items-center gap-2 text-sm font-semibold text-destructive ' \
                 'hover:text-destructive/80',
          data: { action: 'click->dosage-options#remove' }
        ) { 'Remove dose option' }
      end

      def render_removed_dosage_option_state
        div(
          class: 'hidden items-center justify-between gap-4 rounded-shape-xl border border-dashed ' \
                 'border-destructive/30 bg-destructive/5 px-4 py-3',
          data: { 'dosage-options-target': 'removedState' }
        ) do
          Text(size: '2', class: 'font-medium text-foreground') { 'Dose option removed' }
          button(
            type: 'button',
            class: 'inline-flex items-center gap-2 text-sm font-semibold text-foreground hover:text-primary',
            data: { action: 'click->dosage-options#undo' }
          ) { 'Undo' }
        end
      end

      def render_dosage_option_amount_field(dosage, index)
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(for: "medication_dosage_records_attributes_#{index}_amount") do
            'Amount'
          end
          render RubyUI::Input.new(
            type: :number,
            name: dosage_field_name(index, 'amount'),
            id: "medication_dosage_records_attributes_#{index}_amount",
            value: dosage.amount&.to_s,
            step: 'any',
            min: '0',
            required: dosage_row_requires_input?(dosage),
            **field_error_attributes(dosage, :amount, input_id: "medication_dosage_records_attributes_#{index}_amount")
          )
          render_field_error(dosage, :amount, input_id: "medication_dosage_records_attributes_#{index}_amount")
        end
      end

      def render_dosage_option_unit_field(_dosage, index)
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(for: "medication_dosage_records_attributes_#{index}_display_unit") do
            'Unit'
          end
          input(
            type: :hidden,
            name: dosage_field_name(index, 'unit'),
            id: "medication_dosage_records_attributes_#{index}_unit",
            value: medication.dosage_unit
          )
          render RubyUI::Input.new(
            type: :text,
            id: "medication_dosage_records_attributes_#{index}_display_unit",
            value: medication.dosage_unit,
            disabled: true,
            readonly: true,
            class: 'rounded-md border-border bg-muted/70 text-muted-foreground py-4 px-4'
          )
          Text(size: '1', class: 'text-muted-foreground') do
            'Matches the main dose unit'
          end
        end
      end

      def render_dosage_default_number_field(dosage:, index:, config:)
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
            required: dosage_row_requires_input?(dosage),
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
          render RubyUI::Input.new(**options)
          render_field_error(dosage, field, input_id: "medication_dosage_records_attributes_#{index}_#{field}")
        end
      end

      def render_dosage_default_cycle_field(dosage, index)
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(for: "medication_dosage_records_attributes_#{index}_default_dose_cycle") do
            'Dose cycle'
          end
          select(
            name: dosage_field_name(index, 'default_dose_cycle'),
            id: "medication_dosage_records_attributes_#{index}_default_dose_cycle",
            class: 'flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-sm',
            required: dosage_row_requires_input?(dosage),
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

      def render_dosage_default_checkbox(dosage, index, field, label)
        label(class: 'flex items-center gap-2 text-sm cursor-pointer') do
          input(
            type: 'checkbox',
            name: dosage_field_name(index, field),
            value: '1',
            checked: dosage.public_send("#{field}?"),
            class: 'rounded border-input'
          )
          span { label }
        end
      end

      def dosage_field_name(index, field)
        "medication[dosage_records_attributes][#{index}][#{field}]"
      end

      def dosage_row_requires_input?(dosage)
        return true if dosage.persisted? || dosage.errors.any?

        dosage_row_values(dosage).any?(&:present?) || dosage_row_default_flags(dosage).any?
      end

      def dosage_row_values(dosage)
        [
          dosage.amount,
          dosage.frequency,
          dosage.description,
          dosage.default_max_daily_doses,
          dosage.default_min_hours_between_doses
        ]
      end

      def dosage_row_default_flags(dosage)
        [
          dosage.default_for_adults?,
          dosage.default_for_children?
        ]
      end

      def render_current_supply_field
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(
            for: 'medication_current_supply',
            class: 'text-[10px] font-black uppercase tracking-widest text-muted-foreground ml-1'
          ) { t('forms.medications.current_supply') }
          render RubyUI::Input.new(
            type: :number,
            name: 'medication[current_supply]',
            id: 'medication_current_supply',
            value: medication.current_supply,
            min: '0',
            placeholder: t('forms.medications.current_supply_placeholder', default: 'e.g., 30'),
            class: 'rounded-md border-border bg-card py-4 px-4 ' \
                   'focus:ring-2 focus:ring-primary/10 ' \
                   'focus:border-primary transition-all'
          )
        end
      end

      def render_reorder_threshold_field
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(
            for: 'medication_reorder_threshold',
            class: 'text-[10px] font-black uppercase tracking-widest text-muted-foreground ml-1'
          ) { t('forms.medications.reorder_threshold') }
          render RubyUI::Input.new(
            type: :number,
            name: 'medication[reorder_threshold]',
            id: 'medication_reorder_threshold',
            value: medication.reorder_threshold,
            min: '1',
            placeholder: t('forms.medications.reorder_threshold_placeholder', default: 'e.g., 5'),
            class: 'rounded-md border-border bg-card py-4 px-4 ' \
                   'focus:ring-2 focus:ring-primary/10 ' \
                   'focus:border-primary transition-all'
          )
        end
      end

      def render_warnings_field(_form)
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(
            for: 'medication_warnings',
            class: 'text-[10px] font-black uppercase tracking-widest text-error ml-1'
          ) { t('forms.medications.warnings') }
          render RubyUI::Textarea.new(
            name: 'medication[warnings]',
            id: 'medication_warnings',
            rows: 3,
            placeholder: t('forms.medications.warnings_placeholder'),
            class: 'rounded-shape-sm border-error/20 bg-error-container/30 p-4 text-on-error-container focus:ring-2 ' \
                   'focus:ring-error/10 focus:border-error transition-all resize-none ' \
                   'placeholder:text-on-error-container/50'
          ) { medication.warnings }
        end
      end
    end
  end
end
