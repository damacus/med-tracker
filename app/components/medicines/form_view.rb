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
        div(class: 'container mx-auto px-4 py-8 max-w-3xl') do
          render_header
          render_form
        end
      end

      private

      def render_header
        div(class: 'mb-8 text-center sm:text-left') do
          Heading(level: 1, class: 'mb-2') { title }
          Text(weight: 'muted') { subtitle } if subtitle
        end
      end

      def render_form
        form_with(
          model: medicine,
          class: 'space-y-8',
          data: { testid: 'medicine-form' }
        ) do |form|
          render_errors(form) if medicine.errors.any?
          render_form_fields(form)
          render_form_actions(form)
        end
      end

      def render_errors(_form)
        render RubyUI::Alert.new(variant: :destructive, class: 'mb-6') do
          div do
            Heading(level: 2, size: '3', class: 'font-semibold mb-2') do
              plain t('forms.medicines.validation_errors', count: medicine.errors.count)
            end
            ul(class: 'my-2 ml-6 list-disc [&>li]:mt-1') do
              medicine.errors.full_messages.each do |message|
                li { message }
              end
            end
          end
        end
      end

      def render_form_fields(form)
        Card do
          CardContent(class: 'pt-6') do
            div(class: 'grid gap-6 sm:grid-cols-2') do
              render_name_field(form)
              render_description_field(form)
              render_dosage_fields(form)
              render_supply_fields(form)
              render_warnings_field(form)
            end
          end
        end
      end

      def render_name_field(_form)
        div(class: 'sm:col-span-2') do
          FormField do
            FormFieldLabel(for: 'medicine_name') { t('forms.medicines.name') }
            Input(
              type: :text,
              name: 'medicine[name]',
              id: 'medicine_name',
              value: medicine.name,
              required: true,
              class: field_error_class(medicine, :name)
            )
            render_field_error(medicine, :name)
          end
        end
      end

      def render_description_field(_form)
        div(class: 'sm:col-span-2') do
          FormField do
            FormFieldLabel(for: 'medicine_description') { t('forms.medicines.description') }
            Textarea(
              name: 'medicine[description]',
              id: 'medicine_description',
              rows: 3
            ) { medicine.description }
          end
        end
      end

      def render_dosage_fields(_form)
        render_dosage_amount_field
        render_dosage_unit_field
      end

      def render_dosage_amount_field
        div do
          FormField do
            FormFieldLabel(for: 'medicine_dosage_amount') { t('forms.medicines.standard_dosage') }
            Input(
              type: :number,
              name: 'medicine[dosage_amount]',
              id: 'medicine_dosage_amount',
              value: medicine.dosage_amount.to_i,
              step: 'any',
              min: '0'
            )
          end
        end
      end

      def render_dosage_unit_field
        div do
          FormField do
            FormFieldLabel(for: 'medicine_dosage_unit') { t('forms.medicines.unit') }
            select(
              name: 'medicine[dosage_unit]',
              id: 'medicine_dosage_unit',
              class: select_classes
            ) do
              option(value: '', selected: medicine.dosage_unit.blank?) { t('forms.medicines.select_unit') }
              dosage_units.each do |unit|
                option(value: unit, selected: medicine.dosage_unit == unit) { unit }
              end
            end
          end
        end
      end

      def dosage_units
        %w[tablet mg ml g mcg IU spray drop]
      end

      def render_supply_fields(_form)
        render_current_supply_field
        render_stock_field
        render_reorder_threshold_field
      end

      def render_current_supply_field
        div do
          FormField do
            FormFieldLabel(for: 'medicine_current_supply') { t('forms.medicines.current_supply') }
            Input(
              type: :number,
              name: 'medicine[current_supply]',
              id: 'medicine_current_supply',
              value: medicine.current_supply,
              min: '0'
            )
          end
        end
      end

      def render_stock_field
        div do
          FormField do
            FormFieldLabel(for: 'medicine_stock') { t('forms.medicines.stock') }
            Input(
              type: :number,
              name: 'medicine[stock]',
              id: 'medicine_stock',
              value: medicine.stock,
              min: '0'
            )
          end
        end
      end

      def render_reorder_threshold_field
        div do
          FormField do
            FormFieldLabel(for: 'medicine_reorder_threshold') { t('forms.medicines.reorder_threshold') }
            Input(
              type: :number,
              name: 'medicine[reorder_threshold]',
              id: 'medicine_reorder_threshold',
              value: medicine.reorder_threshold,
              min: '1'
            )
          end
        end
      end

      def render_warnings_field(_form)
        div(class: 'sm:col-span-2') do
          FormField do
            FormFieldLabel(for: 'medicine_warnings') { t('forms.medicines.warnings') }
            Textarea(
              name: 'medicine[warnings]',
              id: 'medicine_warnings',
              rows: 3
            ) { medicine.warnings }
          end
        end
      end

      def render_form_actions(_form)
        div(class: 'flex items-center justify-between') do
          Link(href: medicines_path, variant: :outline) { t('forms.medicines.back_to_medicines') }
          Button(type: :submit, variant: :primary) { t('forms.medicines.save_medicine') }
        end
      end
    end
  end
end
