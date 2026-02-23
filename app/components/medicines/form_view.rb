# frozen_string_literal: true

module Components
  module Medicines
    class FormView < Components::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::Pluralize

      attr_reader :medicine, :title, :subtitle

      def initialize(medicine:, title:, subtitle: nil)
        @medicine = medicine
        @title = title
        @subtitle = subtitle
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
            t('forms.medicines.inventory_management')
          end
          Heading(level: 1, size: '8', class: 'font-black tracking-tight text-slate-900') { title }
          Text(size: '3', class: 'text-slate-400') { subtitle } if subtitle
        end
      end

      def render_form
        form_with(
          model: medicine,
          class: 'space-y-8',
          data: { testid: 'medicine-form' }
        ) do |form|
          render_errors(form) if medicine.errors.any?

          Card(class: 'overflow-hidden border-none shadow-2xl rounded-[2.5rem] bg-white') do
            div(class: 'p-10 space-y-8') do
              div(class: 'space-y-6') do
                render_name_field(form)
                render_description_field(form)
              end

              div(class: 'h-px bg-slate-100 w-full')

              div(class: 'space-y-6') do
                Heading(level: 3, size: '4', class: 'font-bold text-slate-900') do
                  t('forms.medicines.dosage_and_supply')
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
              Link(href: medicines_path, variant: :ghost, class: 'font-bold text-slate-400 hover:text-slate-600') do
                t('forms.medicines.back_to_medicines')
              end
              Button(type: :submit, variant: :primary, size: :lg,
                     class: 'px-8 rounded-2xl shadow-lg shadow-primary/20') do
                t('forms.medicines.save_medicine')
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
                plain t('forms.medicines.validation_errors', count: medicine.errors.count)
              end
              ul(class: 'text-sm opacity-90 list-disc pl-4 space-y-1') do
                medicine.errors.full_messages.each do |message|
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

      def render_name_field(_form)
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(
            for: 'medicine_name',
            class: 'text-[10px] font-black uppercase tracking-widest text-slate-400 ml-1'
          ) { t('forms.medicines.name') }
          render RubyUI::Input.new(
            type: :text,
            name: 'medicine[name]',
            id: 'medicine_name',
            value: medicine.name,
            required: true,
            class: 'rounded-2xl border-slate-200 bg-white py-4 px-4 focus:ring-2 focus:ring-primary/10 ' \
                   "focus:border-primary transition-all #{field_error_class(medicine, :name)}"
          )
          render_field_error(medicine, :name)
        end
      end

      def render_description_field(_form)
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(
            for: 'medicine_description',
            class: 'text-[10px] font-black uppercase tracking-widest text-slate-400 ml-1'
          ) { t('forms.medicines.description') }
          render RubyUI::Textarea.new(
            name: 'medicine[description]',
            id: 'medicine_description',
            rows: 3,
            class: 'rounded-2xl border-slate-200 bg-white p-4 focus:ring-2 focus:ring-primary/10 ' \
                   'focus:border-primary transition-all resize-none'
          ) { medicine.description }
        end
      end

      def render_dosage_fields(_form)
        render_dosage_amount_field
        render_dosage_unit_field
      end

      def render_dosage_amount_field
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(
            for: 'medicine_dosage_amount',
            class: 'text-[10px] font-black uppercase tracking-widest text-slate-400 ml-1'
          ) { t('forms.medicines.standard_dosage') }
          render RubyUI::Input.new(
            type: :number,
            name: 'medicine[dosage_amount]',
            id: 'medicine_dosage_amount',
            value: medicine.dosage_amount.to_i,
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
            for: 'medicine_dosage_unit',
            class: 'text-[10px] font-black uppercase tracking-widest text-slate-400 ml-1'
          ) { t('forms.medicines.unit') }
          select(
            name: 'medicine[dosage_unit]',
            id: 'medicine_dosage_unit',
            class: 'flex w-full rounded-2xl border border-slate-200 bg-white py-4 px-4 text-sm ' \
                   'focus:outline-none focus:ring-2 focus:ring-primary/10 focus:border-primary ' \
                   'transition-all appearance-none'
          ) do
            option(value: '', selected: medicine.dosage_unit.blank?) { t('forms.medicines.select_unit') }
            dosage_units.each do |unit|
              option(value: unit, selected: medicine.dosage_unit == unit) { unit }
            end
          end
        end
      end

      def dosage_units
        Medicine::DOSAGE_UNITS
      end

      def render_supply_fields(_form)
        render_current_supply_field
        render_stock_field
        render_reorder_threshold_field
      end

      def render_current_supply_field
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(
            for: 'medicine_current_supply',
            class: 'text-[10px] font-black uppercase tracking-widest text-slate-400 ml-1'
          ) { t('forms.medicines.current_supply') }
          render RubyUI::Input.new(
            type: :number,
            name: 'medicine[current_supply]',
            id: 'medicine_current_supply',
            value: medicine.current_supply,
            min: '0',
            class: 'rounded-2xl border-slate-200 bg-white py-4 px-4 focus:ring-2 focus:ring-primary/10 ' \
                   'focus:border-primary transition-all'
          )
        end
      end

      def render_stock_field
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(
            for: 'medicine_stock',
            class: 'text-[10px] font-black uppercase tracking-widest text-slate-400 ml-1'
          ) { t('forms.medicines.stock') }
          render RubyUI::Input.new(
            type: :number,
            name: 'medicine[stock]',
            id: 'medicine_stock',
            value: medicine.stock,
            min: '0',
            class: 'rounded-2xl border-slate-200 bg-white py-4 px-4 focus:ring-2 focus:ring-primary/10 ' \
                   'focus:border-primary transition-all'
          )
        end
      end

      def render_reorder_threshold_field
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(
            for: 'medicine_reorder_threshold',
            class: 'text-[10px] font-black uppercase tracking-widest text-slate-400 ml-1'
          ) { t('forms.medicines.reorder_threshold') }
          render RubyUI::Input.new(
            type: :number,
            name: 'medicine[reorder_threshold]',
            id: 'medicine_reorder_threshold',
            value: medicine.reorder_threshold,
            min: '1',
            class: 'rounded-2xl border-slate-200 bg-white py-4 px-4 focus:ring-2 focus:ring-primary/10 ' \
                   'focus:border-primary transition-all'
          )
        end
      end

      def render_warnings_field(_form)
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(
            for: 'medicine_warnings',
            class: 'text-[10px] font-black uppercase tracking-widest text-rose-400 ml-1'
          ) { t('forms.medicines.warnings') }
          render RubyUI::Textarea.new(
            name: 'medicine[warnings]',
            id: 'medicine_warnings',
            rows: 3,
            class: 'rounded-2xl border-rose-100 bg-rose-50/30 p-4 text-rose-900 focus:ring-2 ' \
                   'focus:ring-rose-500/10 focus:border-rose-500 transition-all resize-none ' \
                   'placeholder:text-rose-300'
          ) { medicine.warnings }
        end
      end
    end
  end
end
