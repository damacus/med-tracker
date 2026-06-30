# frozen_string_literal: true

module Components
  module Medications
    class TakeAction < Components::Base
      include Phlex::Rails::Helpers::FormWith
      include DoseRecordingHelpers

      attr_reader :source, :person, :current_user, :amount, :button_label, :button_variant, :button_size,
                  :button_class, :button_icon, :disabled, :disabled_label, :disabled_icon, :testid,
                  :disabled_testid, :form_class

      def initialize(source:, context:, amount:, button:, state: {})
        @source = source
        @person = context.fetch(:person)
        @current_user = context.fetch(:current_user)
        @amount = amount
        @button_label = button.fetch(:label)
        @button_variant = button.fetch(:variant)
        @button_size = button.fetch(:size, :lg)
        @button_class = button.fetch(:class, '')
        @button_icon = button[:icon]
        @disabled = state.fetch(:disabled, false)
        @disabled_label = state[:label]
        @disabled_icon = state.fetch(:icon, button_icon)
        @testid = button.fetch(:testid)
        @disabled_testid = button.fetch(:disabled_testid, "#{testid}-disabled")
        @form_class = button.fetch(:form_class, 'flex-1')
        super()
      end

      def view_template
        if disabled
          render_disabled_button
        else
          render_take_dialog
        end
      end

      private

      def render_disabled_button
        m3_button(
          variant: :tonal,
          size: button_size,
          disabled: true,
          class: "#{button_class} grayscale",
          data: { testid: disabled_testid, test_id: disabled_testid }
        ) do
          render_button_content(disabled_label, disabled_icon)
        end
      end

      def render_take_dialog
        Dialog(class: form_class) do
          DialogTrigger(class: 'block w-full') do
            m3_button(
              variant: button_variant,
              size: button_size,
              class: button_class,
              data: { testid: testid, test_id: testid }
            ) do
              render_button_content(button_label, button_icon)
            end
          end

          DialogContent(size: :md) do
            DialogHeader do
              DialogTitle { t('medications.take_action.title', default: 'Record dose') }
              DialogDescription do
                t('medications.take_action.description',
                  default: 'Confirm the person, medication, time, and inventory source.')
              end
            end

            form_with(
              url: take_path,
              method: :post,
              class: 'contents',
              data: {
                controller: 'optimistic-take',
                action: 'submit->optimistic-take#submit',
                optimistic_take_loading_label_value: t('medications.take_action.loading'),
                optimistic_take_queued_label_value: t('medications.take_action.queued', default: 'Queued'),
                offline_source_type: offline_source_type,
                offline_source_id: source.id
              }
            ) do
              DialogMiddle do
                div(class: 'space-y-5') do
                  input(type: :hidden, name: 'dose_amount', value: formatted_amount)
                  input(type: :hidden, name: 'dose_unit', value: source_dose_unit)
                  render_context
                  render_taken_at_field
                  render_stock_source_selection
                end
              end

              DialogFooter(class: 'border-t border-border/70 bg-popover px-8 pb-8 pt-4') do
                render M3::Button.new(
                  type: :submit,
                  variant: :filled,
                  class: 'w-full sm:w-auto',
                  data: { optimistic_take_target: 'button' }
                ) do
                  render_button_content(button_label, button_icon)
                end
              end
            end
          end
        end
      end

      def render_button_content(label, icon)
        render icon.new(size: button_icon_size, aria_hidden: 'true', class: 'mr-2 shrink-0') if icon
        plain label
      end

      def button_icon_size
        case button_size
        when :sm then 16
        when :xl then 20
        else 18
        end
      end

      def render_taken_at_field
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(for: taken_at_input_id) do
            t('medications.take_action.taken_at', default: 'Taken at')
          end
          m3_input(
            id: taken_at_input_id,
            type: 'time',
            name: 'medication_take[taken_at]',
            value: time_field_value,
            max: time_field_max
          )
        end
      end

      def taken_at_input_id
        @taken_at_input_id ||= "medication_take_taken_at_#{source.class.name.underscore}_#{source.id}"
      end

      def time_field_value
        @time_field_value ||= Time.current.strftime('%H:%M')
      end

      def time_field_max
        @time_field_max ||= begin
          upper = Time.current + TakeMedicationGuardable::FUTURE_TOLERANCE
          if upper.to_date == Date.current
            upper.strftime('%H:%M')
          else
            '23:59'
          end
        end
      end
    end
  end
end
