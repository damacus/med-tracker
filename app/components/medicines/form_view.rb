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
          h1(class: 'text-3xl font-bold text-slate-900 mb-2') { title }
          p(class: 'text-slate-600') { subtitle } if subtitle
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
            h2(class: 'font-semibold mb-2') do
              plain "#{pluralize(medicine.errors.count, 'error')} prevented this medicine from being saved:"
            end
            ul(class: 'list-disc space-y-1 pl-5') do
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
          render RubyUI::FormField.new do
            render RubyUI::FormFieldLabel.new(for: 'medicine_name') { 'Name' }
            render RubyUI::Input.new(
              type: :text,
              name: 'medicine[name]',
              id: 'medicine_name',
              value: medicine.name,
              required: true
            )
          end
        end
      end

      def render_description_field(_form)
        div(class: 'sm:col-span-2') do
          render RubyUI::FormField.new do
            render RubyUI::FormFieldLabel.new(for: 'medicine_description') { 'Description' }
            textarea(
              name: 'medicine[description]',
              id: 'medicine_description',
              rows: '3',
              class: input_classes
            ) { medicine.description }
          end
        end
      end

      def render_dosage_fields(_form)
        div do
          render RubyUI::FormField.new do
            render RubyUI::FormFieldLabel.new(for: 'medicine_dosage_amount') { 'Standard Dosage' }
            render RubyUI::Input.new(
              type: :number,
              name: 'medicine[dosage_amount]',
              id: 'medicine_dosage_amount',
              value: medicine.dosage_amount,
              step: 'any',
              min: '0'
            )
          end
        end

        div do
          render RubyUI::FormField.new do
            render RubyUI::FormFieldLabel.new(for: 'medicine_dosage_unit') { 'Unit' }
            select(
              name: 'medicine[dosage_unit]',
              id: 'medicine_dosage_unit',
              class: input_classes
            ) do
              option(value: '', selected: medicine.dosage_unit.blank?) { 'Select unit' }
              %w[tablet mg ml g mcg IU spray drop].each do |unit|
                option(value: unit, selected: medicine.dosage_unit == unit) { unit }
              end
            end
          end
        end
      end

      def render_supply_fields(_form)
        div do
          render RubyUI::FormField.new do
            render RubyUI::FormFieldLabel.new(for: 'medicine_current_supply') { 'Current Supply' }
            render RubyUI::Input.new(
              type: :number,
              name: 'medicine[current_supply]',
              id: 'medicine_current_supply',
              value: medicine.current_supply,
              min: '0'
            )
          end
        end

        div do
          render RubyUI::FormField.new do
            render RubyUI::FormFieldLabel.new(for: 'medicine_stock') { 'Stock' }
            render RubyUI::Input.new(
              type: :number,
              name: 'medicine[stock]',
              id: 'medicine_stock',
              value: medicine.stock,
              min: '0'
            )
          end
        end

        div do
          render RubyUI::FormField.new do
            render RubyUI::FormFieldLabel.new(for: 'medicine_reorder_threshold') { 'Reorder Threshold' }
            render RubyUI::Input.new(
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
          render RubyUI::FormField.new do
            render RubyUI::FormFieldLabel.new(for: 'medicine_warnings') { 'Warnings' }
            textarea(
              name: 'medicine[warnings]',
              id: 'medicine_warnings',
              rows: '3',
              class: input_classes
            ) { medicine.warnings }
          end
        end
      end

      def render_form_actions(_form)
        div(class: 'flex items-center justify-between') do
          a(
            href: medicines_path,
            class: button_secondary_classes
          ) { 'Back to Medicines' }

          render RubyUI::Button.new(type: :submit, variant: :primary) { 'Save Medicine' }
        end
      end

      def input_classes
        'w-full rounded-md border border-input bg-background px-3 py-2 text-sm ' \
          'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring'
      end

      def button_secondary_classes
        'inline-flex items-center justify-center rounded-md font-medium transition-colors ' \
          'px-4 py-2 h-10 text-sm border border-input bg-background hover:bg-accent hover:text-accent-foreground'
      end
    end
  end
end
