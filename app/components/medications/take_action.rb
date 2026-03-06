# frozen_string_literal: true

module Components
  module Medications
    class TakeAction < Components::Base
      include Phlex::Rails::Helpers::FormWith

      attr_reader :source, :person, :current_user, :amount, :button_label, :button_variant, :button_size,
                  :button_class, :disabled, :disabled_label, :testid, :disabled_testid, :form_class

      def initialize(source:, context:, amount:, button:, state: {})
        @source = source
        @person = context.fetch(:person)
        @current_user = context.fetch(:current_user)
        @amount = amount
        @button_label = button.fetch(:label)
        @button_variant = button.fetch(:variant)
        @button_size = button.fetch(:size, :lg)
        @button_class = button.fetch(:class, '')
        @disabled = state.fetch(:disabled, false)
        @disabled_label = state[:label]
        @testid = button.fetch(:testid)
        @disabled_testid = button.fetch(:disabled_testid, "#{testid}-disabled")
        @form_class = button.fetch(:form_class, 'flex-1')
        super()
      end

      def view_template
        if disabled
          render_disabled_button
        elsif available_medications.many?
          render_location_dialog
        else
          render_take_form(selected_medication: available_medications.first)
        end
      end

      private

      def render_disabled_button
        render Button.new(
          variant: :secondary,
          size: button_size,
          disabled: true,
          class: "#{button_class} grayscale",
          data: { testid: disabled_testid, test_id: disabled_testid }
        ) { disabled_label }
      end

      def render_location_dialog
        Dialog do
          DialogTrigger do
            render Button.new(
              variant: button_variant,
              size: button_size,
              class: button_class,
              data: { testid: testid, test_id: testid }
            ) { button_label }
          end

          DialogContent(size: :md) do
            DialogHeader do
              DialogTitle { t('medications.take_action.choose_location', default: 'Choose location') }
              DialogDescription do
                t('medications.take_action.choose_location_description',
                  default: 'Select which location to deduct this dose from.')
              end
            end

            DialogMiddle do
              form_with(
                url: take_path,
                method: :post,
                class: 'space-y-4',
                data: { controller: 'optimistic-take', action: 'submit->optimistic-take#submit' }
              ) do
                input(type: :hidden, name: 'amount_ml', value: formatted_amount)

                div(class: 'space-y-2') do
                  available_medications.each_with_index do |medication, index|
                    label(
                      for: "taken_from_medication_#{source.class.name.underscore}_#{source.id}_#{medication.id}",
                      class: 'flex items-center justify-between gap-3 rounded-2xl border border-slate-200 p-4'
                    ) do
                      div(class: 'space-y-1') do
                        Text(size: '2', weight: 'bold', class: 'text-slate-900') { medication.location.name }
                        Text(size: '1', class: 'text-slate-500') { medication_description(medication) }
                      end
                      div(class: 'flex items-center gap-3') do
                        Badge(variant: :outline, class: 'rounded-full text-[10px]') { inventory_label(medication) }
                        input(
                          type: :radio,
                          id: "taken_from_medication_#{source.class.name.underscore}_#{source.id}_#{medication.id}",
                          name: 'taken_from_medication_id',
                          value: medication.id,
                          checked: index.zero?
                        )
                      end
                    end
                  end
                end

                div(class: 'flex justify-end') do
                  render RubyUI::Button.new(
                    type: :submit,
                    variant: :primary,
                    class: 'rounded-xl',
                    data: { optimistic_take_target: 'button' }
                  ) { button_label }
                end
              end
            end
          end
        end
      end

      def render_take_form(selected_medication:)
        form_with(
          url: take_path,
          method: :post,
          class: form_class,
          data: { controller: 'optimistic-take', action: 'submit->optimistic-take#submit' }
        ) do
          input(type: :hidden, name: 'amount_ml', value: formatted_amount)
          input(type: :hidden, name: 'taken_from_medication_id', value: selected_medication.id)
          render RubyUI::Button.new(
            type: :submit,
            variant: button_variant,
            size: button_size,
            class: button_class,
            data: { optimistic_take_target: 'button', testid: testid, test_id: testid }
          ) { button_label }
        end
      end

      def take_path
        if source.is_a?(::Schedule)
          take_medication_person_schedule_path(person, source)
        else
          take_medication_person_person_medication_path(person, source)
        end
      end

      def available_medications
        @available_medications ||= stock_source_resolver.available_medications
      end

      def medication_description(medication)
        parts = [medication.name]
        if medication.dosage_amount.present? && medication.dosage_unit.present?
          parts << "#{medication.dosage_amount.to_f.to_s.sub(/\.0$/, '')}#{medication.dosage_unit}"
        end
        parts.join(' • ')
      end

      def inventory_label(medication)
        if medication.current_supply.blank?
          return t('medications.take_action.untracked_inventory', default: 'Untracked')
        end

        pluralize(medication.current_supply, 'unit')
      end

      def formatted_amount
        amount.to_s
      end

      def stock_source_resolver
        @stock_source_resolver ||= MedicationStockSourceResolver.new(user: current_user, source: source)
      end
    end
  end
end
