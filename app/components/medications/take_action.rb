# frozen_string_literal: true

module Components
  module Medications
    class TakeAction < Components::Base
      include Phlex::Rails::Helpers::FormWith

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
                  class: 'w-full rounded-xl sm:w-auto',
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

      def render_context
        div(class: 'rounded-shape-xl border border-border/70 bg-surface-container-low p-4') do
          dl(class: 'grid gap-3 sm:grid-cols-2') do
            div(class: 'space-y-1') do
              dt(class: 'text-xs font-black uppercase text-on-surface-variant') do
                t('medications.take_action.person', default: 'Person')
              end
              dd(class: 'text-sm font-semibold text-foreground') { person.name }
            end
            div(class: 'space-y-1') do
              dt(class: 'text-xs font-black uppercase text-on-surface-variant') do
                t('medications.take_action.medication', default: 'Medication')
              end
              dd(class: 'text-sm font-semibold text-foreground') { source.medication.name }
            end
            div(class: 'space-y-1 sm:col-span-2') do
              dt(class: 'text-xs font-black uppercase text-on-surface-variant') do
                t('medications.take_action.dose', default: 'Dose')
              end
              dd(class: 'text-sm font-semibold text-foreground') { dose_label }
            end
          end
        end
      end

      def render_taken_at_field
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(for: taken_at_input_id) do
            t('medications.take_action.taken_at', default: 'Taken at')
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

      def render_stock_source_selection
        if available_medications.many?
          render_stock_source_options
        elsif available_medications.one?
          input(type: :hidden, name: 'medication_take[taken_from_medication_id]', value: available_medications.first.id)
          render_selected_stock_source(available_medications.first)
        end
      end

      def render_stock_source_options
        fieldset(class: 'space-y-2') do
          legend(class: 'text-sm font-semibold text-foreground') do
            t('medications.take_action.stock_source', default: 'Stock source')
          end
          available_medications.each_with_index do |medication, index|
            render_stock_source_option(medication, index)
          end
        end
      end

      def render_stock_source_option(medication, index)
        label(
          for: stock_source_input_id(medication),
          class: 'flex flex-col gap-3 rounded-shape-xl border border-border p-4 sm:flex-row ' \
                 'sm:items-center sm:justify-between'
        ) do
          render_stock_source_details(medication)
          div(class: 'flex items-center gap-3 shrink-0') do
            Badge(variant: :outlined, class: 'rounded-full text-[10px] whitespace-nowrap justify-center') do
              inventory_label(medication)
            end
            input(
              type: :radio,
              id: stock_source_input_id(medication),
              name: 'medication_take[taken_from_medication_id]',
              value: medication.id,
              checked: index.zero?
            )
          end
        end
      end

      def render_selected_stock_source(medication)
        div(class: 'space-y-2') do
          m3_text(variant: :label_medium, class: 'font-semibold text-foreground') do
            t('medications.take_action.stock_source', default: 'Stock source')
          end
          div(class: 'rounded-shape-xl border border-border p-4') do
            render_stock_source_details(medication)
          end
        end
      end

      def render_stock_source_details(medication)
        div(class: 'space-y-1 min-w-0') do
          m3_text(size: '2', weight: 'bold', class: 'text-foreground') { medication.location.name }
          m3_text(size: '1', class: 'text-on-surface-variant break-words') do
            medication_description(medication)
          end
        end
      end

      def take_path
        if source.is_a?(::Schedule)
          take_medication_person_schedule_path(person, source)
        else
          take_medication_person_person_medication_path(person, source)
        end
      end

      def offline_source_type
        source.is_a?(::Schedule) ? 'schedule' : 'person_medication'
      end

      def available_medications
        @available_medications ||= stock_source_resolver.available_medications
      end

      def medication_description(medication)
        parts = [medication.name]
        dose = DoseAmount.new(medication.dosage_amount, medication.dosage_unit).to_s
        parts << dose if dose.present?
        parts.join(' • ')
      end

      def inventory_label(medication)
        if medication.current_supply.blank?
          return t('medications.take_action.untracked_inventory', default: 'Untracked')
        end

        stock_label_for(medication)
      end

      def stock_label_for(medication)
        if medication.dosage_unit == 'ml'
          return "#{MedicationStockQuantityFormatter.format(medication.current_supply)} ml"
        end

        supply = MedicationStockQuantityFormatter.format(medication.current_supply)
        supply == '1' ? '1 unit' : "#{supply} units"
      end

      def formatted_amount
        amount.to_s
      end

      def dose_label
        dose = DoseAmount.new(amount, source_dose_unit).to_s
        dose.presence || formatted_amount
      end

      def source_dose_unit
        return source.dose_unit if source.respond_to?(:dose_unit)

        source.medication.dosage_unit
      end

      def taken_at_input_id
        @taken_at_input_id ||= "medication_take_taken_at_#{source.class.name.underscore}_#{source.id}"
      end

      def stock_source_input_id(medication)
        "taken_from_medication_#{source.class.name.underscore}_#{source.id}_#{medication.id}"
      end

      def taken_at_field_value
        @taken_at_field_value ||= Time.current.strftime('%Y-%m-%dT%H:%M')
      end

      def stock_source_resolver
        @stock_source_resolver ||= MedicationStockSourceResolver.new(user: current_user, source: source)
      end
    end
  end
end
