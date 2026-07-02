# frozen_string_literal: true

module Components
  module Medications
    class IdentityFields < Components::Base
      attr_reader :medication, :locations

      def initialize(medication:, locations:)
        @medication = medication
        @locations = locations
        super()
      end

      def view_template
        render_location_field
        render_name_field
        render_friendly_name_field
        render_category_field
        render_description_field
      end

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
              class: "rounded-shape-sm #{field_error_class(medication, :location)}"
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
            class: 'rounded-shape-sm border-outline-variant bg-surface-container-lowest py-4 px-4 ' \
                   'focus:ring-2 focus:ring-primary/10 ' \
                   "focus:border-primary transition-all #{field_error_class(medication, :name)}",
            **field_error_attributes(medication, :name, input_id: 'medication_name')
          )
          render_field_error(medication, :name, input_id: 'medication_name')
        end
      end

      def render_friendly_name_field
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(
            for: 'medication_friendly_name',
            class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1'
          ) { t('forms.medications.friendly_name') }
          m3_input(
            type: :text,
            name: 'medication[friendly_name]',
            id: 'medication_friendly_name',
            value: medication.friendly_name,
            placeholder: t('forms.medications.friendly_name_placeholder'),
            class: 'rounded-shape-sm border-outline-variant bg-surface-container-lowest py-4 px-4 ' \
                   'focus:ring-2 focus:ring-primary/10 focus:border-primary transition-all'
          )
          m3_text(variant: :body_small, class: 'text-on-surface-variant ml-1') do
            t('forms.medications.friendly_name_hint')
          end
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
              class: "rounded-shape-sm #{field_error_class(medication, :category)}"
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
            class: 'rounded-shape-sm border-outline-variant bg-surface-container-lowest p-4 ' \
                   'focus:ring-2 focus:ring-primary/10 ' \
                   'focus:border-primary transition-all resize-none'
          ) { medication.description }
        end
      end

      def selected_location_name
        return nil if medication.location_id.blank?

        locations.find { |l| l.id == medication.location_id }&.name
      end
    end
  end
end
