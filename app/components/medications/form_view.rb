# frozen_string_literal: true

module Components
  module Medications
    class FormView < Components::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::Pluralize

      attr_reader :medication, :title, :subtitle, :locations, :return_to

      def initialize(medication:, title:, subtitle: nil, locations: Location.order(:name), return_to: nil)
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
          Text(size: '2', weight: 'black', class: 'uppercase tracking-[0.2em] font-bold opacity-40') do
            t('forms.medications.inventory_management')
          end
          Heading(level: 1, size: '8', class: 'font-black tracking-tight text-slate-900') { title }
          Text(size: '3', class: 'text-slate-400') { subtitle } if subtitle
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

          Card(class: 'overflow-hidden border-none shadow-2xl rounded-[2.5rem] bg-white') do
            div(class: 'p-10 space-y-8') do
              div(class: 'space-y-6') do
                render_location_field(form)
                render_name_field(form)
                render_category_field(form)
                render_description_field(form)
              end

              div(class: 'h-px bg-slate-100 w-full')

              div(class: 'space-y-6') do
                Heading(level: 3, size: '4', class: 'font-bold text-slate-900') do
                  t('forms.medications.dosage_and_supply')
                end
                div(class: 'grid grid-cols-1 sm:grid-cols-2 gap-6') do
                  render_dosage_fields(form)
                  render_supply_fields(form)
                end
              end

              div(class: 'h-px bg-slate-100 w-full')

              render_warnings_field(form)
            end

            div(class: 'px-10 py-6 bg-slate-50/50 border-t border-slate-100 flex items-center justify-between gap-4') do
              Link(href: return_to.presence || medications_path, variant: :ghost,
                   class: 'font-bold text-slate-400 hover:text-slate-600') do
                t('forms.medications.back')
              end
              Button(type: :submit, variant: :primary, size: :lg,
                     class: 'px-8 rounded-2xl shadow-lg shadow-primary/20') do
                t('forms.medications.save_medication')
              end
            end
          end
        end
      end

      def render_errors(_form)
        render RubyUI::Alert.new(variant: :destructive, class: 'mb-8 rounded-2xl border-none shadow-sm') do
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
            for: 'medication_location_id',
            class: 'text-[10px] font-black uppercase tracking-widest text-slate-400 ml-1'
          ) { t('medications.show.location') }
          select(
            name: 'medication[location_id]',
            id: 'medication_location_id',
            required: true,
            class: "#{select_classes} #{field_error_class(medication, :location)}"
          ) do
            option(value: '') { t('forms.medications.select_location') }
            locations.each do |loc|
              option(value: loc.id, selected: medication.location_id == loc.id) { loc.name }
            end
          end
          render_field_error(medication, :location)
        end
      end

      def render_name_field(_form)
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
            class: 'rounded-2xl border-slate-200 bg-white py-4 px-4 focus:ring-2 focus:ring-primary/10 ' \
                   "focus:border-primary transition-all #{field_error_class(medication, :name)}"
          )
          render_field_error(medication, :name)
        end
      end

      def render_category_field(_form)
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(
            for: 'medication_category',
            class: 'text-[10px] font-black uppercase tracking-widest text-slate-400 ml-1'
          ) { 'Category' }
          select(
            name: 'medication[category]',
            id: 'medication_category',
            class: "#{select_classes} #{field_error_class(medication, :category)}"
          ) do
            option(value: '') { t('forms.medications.select_category') }
            Medication::CATEGORIES.each do |cat|
              option(value: cat, selected: medication.category == cat) { cat.titleize }
            end
          end
          render_field_error(medication, :category)
        end
      end

      def render_description_field(_form)
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(
            for: 'medication_description',
            class: 'text-[10px] font-black uppercase tracking-widest text-slate-400 ml-1'
          ) { t('forms.medications.description') }
          render RubyUI::Textarea.new(
            name: 'medication[description]',
            id: 'medication_description',
            rows: 3,
            class: 'rounded-2xl border-slate-200 bg-white p-4 focus:ring-2 focus:ring-primary/10 ' \
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
            class: 'text-[10px] font-black uppercase tracking-widest text-slate-400 ml-1'
          ) { t('forms.medications.standard_dosage') }
          render RubyUI::Input.new(
            type: :number,
            name: 'medication[dosage_amount]',
            id: 'medication_dosage_amount',
            value: medication.dosage_amount.to_i,
            step: 'any',
            min: '0',
            class: 'rounded-2xl border-slate-200 bg-white py-4 px-4 focus:ring-2 focus:ring-primary/10 ' \
                   'focus:border-primary transition-all'
          )
        end
      end

      def render_dosage_unit_field
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(
            for: 'medication_dosage_unit',
            class: 'text-[10px] font-black uppercase tracking-widest text-slate-400 ml-1'
          ) { t('forms.medications.unit') }
          select(
            name: 'medication[dosage_unit]',
            id: 'medication_dosage_unit',
            class: select_classes
          ) do
            option(value: '', selected: medication.dosage_unit.blank?) { t('forms.medications.select_unit') }
            dosage_units.each do |unit|
              option(value: unit, selected: medication.dosage_unit == unit) { unit }
            end
          end
        end
      end

      def dosage_units
        Medication::DOSAGE_UNITS
      end

      def render_supply_fields(_form)
        render_current_supply_field
        render_reorder_threshold_field
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
            class: 'rounded-2xl border-slate-200 bg-white py-4 px-4 focus:ring-2 focus:ring-primary/10 ' \
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
            class: 'rounded-2xl border-slate-200 bg-white py-4 px-4 focus:ring-2 focus:ring-primary/10 ' \
                   'focus:border-primary transition-all'
          )
        end
      end

      def render_warnings_field(_form)
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(
            for: 'medication_warnings',
            class: 'text-[10px] font-black uppercase tracking-widest text-rose-400 ml-1'
          ) { t('forms.medications.warnings') }
          render RubyUI::Textarea.new(
            name: 'medication[warnings]',
            id: 'medication_warnings',
            rows: 3,
            class: 'rounded-2xl border-rose-100 bg-rose-50/30 p-4 text-rose-900 focus:ring-2 ' \
                   'focus:ring-rose-500/10 focus:border-rose-500 transition-all resize-none ' \
                   'placeholder:text-rose-300'
          ) { medication.warnings }
        end
      end
    end
  end
end
