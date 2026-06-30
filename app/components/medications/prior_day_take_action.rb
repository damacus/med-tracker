# frozen_string_literal: true

module Components
  module Medications
    class PriorDayTakeAction < Components::Base
      include Phlex::Rails::Helpers::FormWith
      include DoseRecordingHelpers

      attr_reader :source, :person, :current_user, :amount, :testid, :button_options

      def initialize(source:, context:, amount:, testid:, button: nil)
        @source = source
        @person = context.fetch(:person)
        @current_user = context.fetch(:current_user)
        @amount = amount
        @testid = testid
        @button_options = button
        super()
      end

      def view_template
        Dialog(class: button_options ? nil : 'block w-full') do
          DialogTrigger(class: button_options ? nil : 'block w-full') { render_trigger }
          DialogContent(size: :md) do
            DialogHeader do
              DialogTitle do
                t('medications.prior_day_take_action.title',
                  default: 'Record a dose from a previous day')
              end
              DialogDescription do
                t('medications.prior_day_take_action.description',
                  default: 'Backdate a dose taken before today.')
              end
            end

            form_with(url: take_path, method: :post, class: 'contents') do
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
                  class: 'w-full sm:w-auto'
                ) do
                  plain t('medications.prior_day_take_action.submit',
                          default: 'Record historical dose')
                end
              end
            end
          end
        end
      end

      private

      def render_trigger
        if button_options
          render_button_trigger
        else
          render_menu_item_trigger
        end
      end

      def render_button_trigger
        m3_button(
          variant: button_options[:variant] || :outlined,
          size: button_options[:size] || :md,
          class: button_options[:class],
          data: {
            action: 'click->ruby-ui--dialog#open:prevent',
            testid: testid
          }
        ) do
          render Icons::Calendar.new(size: 16, aria_hidden: 'true', class: 'mr-2 shrink-0')
          plain t('medications.prior_day_take_action.menu_item', default: 'Log a past dose')
        end
      end

      def render_menu_item_trigger
        render RubyUI::DropdownMenuItem.new(
          href: '#',
          data_action: 'click->ruby-ui--dialog#open:prevent',
          data_testid: testid,
          data_test_id: testid
        ) do
          render Icons::Calendar.new(size: 16, aria_hidden: 'true', class: 'mr-2 shrink-0')
          plain t('medications.prior_day_take_action.menu_item', default: 'Log a past dose')
        end
      end

      def render_taken_at_field
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(for: taken_at_input_id) do
            t('medications.prior_day_take_action.taken_at', default: 'Date and time taken')
          end
          m3_input(
            id: taken_at_input_id,
            type: 'datetime-local',
            name: 'medication_take[taken_at]',
            value: taken_at_field_value,
            max: taken_at_field_value
          )
        end
      end

      def taken_at_input_id
        @taken_at_input_id ||= "medication_take_taken_at_prior_day_#{source.class.name.underscore}_#{source.id}"
      end

      def taken_at_field_value
        @taken_at_field_value ||= Time.current.strftime('%Y-%m-%dT%H:%M')
      end
    end
  end
end
