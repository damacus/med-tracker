# frozen_string_literal: true

module Components
  module Medications
    class StockCheckView < Components::Base
      attr_reader :medications, :locations, :current_location_id, :initial_adjustments, :reason, :error

      def initialize(medications:, locations:, current_location_id: nil, state: {})
        @medications = medications
        @locations = locations
        @current_location_id = current_location_id
        @initial_adjustments = state.fetch(:initial_adjustments, {}).to_h.stringify_keys
        @reason = state[:reason]
        @error = state[:error]
        super()
      end

      def view_template
        form_with(
          url: bulk_adjust_inventory_medications_path(location_id: current_location_id),
          method: :patch,
          class: 'min-h-[calc(100vh-4rem)] min-w-0 bg-surface',
          data: stock_check_controller_data
        ) do
          render_header
          render_error
          render_workspace
        end
      end

      private

      def stock_check_controller_data
        {
          controller: 'stock-check',
          stock_check_apply_label_value: t('medications.stock_check.apply', count: '__COUNT__'),
          stock_check_selected_label_value: t('medications.stock_check.selected', count: '__COUNT__'),
          stock_check_medicines_label_value: t('medications.stock_check.medicines', count: '__COUNT__'),
          stock_check_units_value: t('medications.stock_check.units')
        }
      end

      def render_header
        header(class: 'border-b border-border px-4 py-8 md:px-8 lg:px-10') do
          div(class: 'mx-auto flex max-w-[90rem] flex-col gap-6 lg:flex-row lg:items-center lg:justify-between') do
            div(class: 'space-y-2') do
              m3_text(
                variant: :label_medium,
                class: 'font-black uppercase tracking-[0.2em] text-primary'
              ) { t('medications.stock_check.eyebrow', location: selected_location_name) }
              div(class: 'flex flex-wrap items-center gap-3') do
                m3_heading(
                  level: 1,
                  variant: :display_small,
                  class: 'font-black tracking-tight text-on-surface'
                ) { t('medications.stock_check.title') }
                render_mode_badge
              end
              m3_text(variant: :body_large, class: 'text-on-surface-variant') do
                t('medications.stock_check.description')
              end
            end
            div(class: 'flex items-center gap-2 text-sm font-semibold text-on-surface-variant') do
              render Icons::Calendar.new(size: 20)
              time(datetime: Date.current.iso8601) { I18n.l(Date.current, format: :long) }
            end
          end
        end
      end

      def render_mode_badge
        m3_badge(
          variant: :outline,
          class: 'gap-2 border-primary/30 bg-primary-container/40 px-3 py-1 text-primary'
        ) do
          render Icons::Pencil.new(size: 15)
          span { t('medications.stock_check.mode') }
        end
      end

      def render_error
        return if error.blank?

        div(class: 'mx-auto max-w-[90rem] px-4 pt-6 md:px-8 lg:px-10') do
          render RubyUI::Alert.new(variant: :destructive, class: 'rounded-shape-lg') do
            plain error
          end
        end
      end

      def render_workspace
        div(
          class: 'mx-auto grid w-full min-w-0 max-w-[90rem] ' \
                 'lg:grid-cols-[minmax(20rem,0.85fr)_minmax(0,1.4fr)]'
        ) do
          render_medicine_picker
          render_amendment_batch
        end
      end

      def render_medicine_picker
        section(class: 'min-w-0 border-b border-border lg:border-b-0 lg:border-r') do
          div(class: 'space-y-6 px-4 py-8 md:px-8') do
            render_section_heading(
              title: t('medications.stock_check.choose_title'),
              description: t('medications.stock_check.choose_description')
            )
            render_picker_controls
          end
          render_selection_toolbar
          render_medicine_rows
        end
      end

      def render_section_heading(title:, description:)
        div(class: 'space-y-1') do
          m3_heading(level: 2, variant: :title_large, class: 'font-bold') { title }
          m3_text(variant: :body_medium, class: 'text-on-surface-variant') { description }
        end
      end

      def render_picker_controls
        div(class: 'grid gap-3 sm:grid-cols-[minmax(0,1fr)_9rem]') do
          FormField do
            FormFieldLabel(for: 'stock_check_search', class: 'sr-only') do
              t('medications.stock_check.search_label')
            end
            div(class: 'relative') do
              div(class: 'pointer-events-none absolute inset-y-0 left-4 flex items-center text-on-surface-variant') do
                render Icons::Search.new(size: 19)
              end
              m3_input(
                id: 'stock_check_search',
                type: :search,
                placeholder: t('medications.stock_check.search_placeholder'),
                class: 'h-12 min-h-12 py-3 pl-11',
                data: {
                  stock_check_target: 'search',
                  action: 'input->stock-check#filterMedicines'
                }
              )
            end
          end
          FormField do
            FormFieldLabel(for: 'stock_check_location', class: 'sr-only') do
              t('medications.stock_check.location_label')
            end
            m3_select(
              id: 'stock_check_location',
              size: :sm,
              class: 'h-12 min-h-12 bg-surface-container-lowest',
              data: { action: 'change->stock-check#changeLocation' }
            ) do
              option(value: stock_check_medications_path(location_id: '')) do
                t('medications.stock_check.all_locations')
              end
              locations.each do |location|
                option(
                  value: stock_check_medications_path(location_id: location.id),
                  selected: current_location_id.to_i == location.id
                ) { location.name }
              end
            end
          end
        end
      end

      def render_selection_toolbar
        div(class: 'flex min-h-14 items-center justify-between border-y border-border px-4 md:px-8') do
          span(
            class: 'font-bold text-primary',
            data: { stock_check_target: 'selectedCount' },
            aria_live: 'polite'
          ) { t('medications.stock_check.selected', count: initial_adjustments.size) }
          m3_button(
            type: :button,
            variant: :text,
            size: :sm,
            data: { action: 'stock-check#clearSelection' }
          ) { t('medications.stock_check.clear_selection') }
        end
      end

      def render_medicine_rows
        div(class: 'divide-y divide-border lg:max-h-[42rem] lg:overflow-y-auto') do
          medications.each { |medication| render_medicine_row(medication) }
        end
      end

      def render_medicine_row(medication)
        selected = selected?(medication)
        label(
          for: selection_id(medication),
          class: medicine_row_classes(selected),
          data: {
            stock_check_target: 'medicineRow',
            medication_name: medication.display_name.downcase,
            medication_id: medication.id
          }
        ) do
          input(
            id: selection_id(medication),
            type: 'checkbox',
            checked: selected,
            class: 'size-5 shrink-0 rounded border-outline accent-primary',
            data: {
              stock_check_target: 'selection',
              action: 'change->stock-check#toggleMedication',
              stock_check_id_param: medication.id
            }
          )
          render_medication_identity(medication)
          div(class: 'ml-auto text-right') do
            div(class: 'font-bold text-on-surface') { supply_label(medication) }
            render_supply_badge(medication)
          end
        end
      end

      def medicine_row_classes(selected)
        classes = [
          'flex min-h-24 cursor-pointer items-center gap-4 px-4 py-4 transition-colors md:px-8',
          'hover:bg-primary-container/20 focus-within:bg-primary-container/20'
        ]
        classes << 'bg-primary-container/25' if selected
        classes
      end

      def render_medication_identity(medication)
        div(class: 'flex min-w-0 items-center gap-3') do
          div(
            class: 'flex size-11 shrink-0 items-center justify-center rounded-shape-full ' \
                   'bg-secondary-container text-primary'
          ) { render Components::Shared::MedicationIcon.new(medication: medication, size: 21) }
          div(class: 'min-w-0') do
            div(class: 'truncate font-bold text-on-surface') { medication.display_name }
            m3_text(variant: :body_small, class: 'text-on-surface-variant') { dosage_label(medication) }
          end
        end
      end

      def render_supply_badge(medication)
        label, variant = supply_status(medication)
        m3_badge(variant: variant, size: :sm, class: 'mt-1') { label }
      end

      def supply_status(medication)
        return [t('medications.stock_check.out_of_stock'), :destructive] if current_supply(medication).zero?
        return [t('medications.stock_check.low_stock'), :warning] if medication.low_stock?

        [t('medications.stock_check.in_stock'), :success]
      end

      def render_amendment_batch
        section(class: 'flex min-w-0 flex-col bg-surface-container-lowest/70') do
          div(class: 'px-4 py-8 md:px-8') do
            render_section_heading(
              title: t('medications.stock_check.batch_title'),
              description: t('medications.stock_check.batch_description')
            )
            div(class: 'mt-14') { render_batch_table }
            div(class: 'mt-10') { render_reason_field }
          end
          render_batch_actions
        end
      end

      def render_batch_table
        div do
          render_batch_headers
          div(class: 'overflow-hidden rounded-shape-lg border border-border bg-card shadow-elevation-1') do
            render_empty_batch
            div(class: 'divide-y divide-border') do
              medications.each { |medication| render_batch_row(medication) }
            end
          end
        end
      end

      def render_batch_headers
        div(
          class: 'hidden grid-cols-[minmax(8rem,1fr)_4.5rem_minmax(9rem,1fr)_5.5rem_8rem] ' \
                 'gap-3 px-4 py-3 text-xs font-bold text-on-surface-variant xl:grid'
        ) do
          span { t('medications.stock_check.medicine') }
          span { t('medications.stock_check.current') }
          span { t('medications.stock_check.new_supply') }
          span { t('medications.stock_check.difference') }
          span(class: 'sr-only') { t('medications.stock_check.actions') }
        end
      end

      def render_empty_batch
        div(
          class: initial_adjustments.empty? ? 'px-6 py-16 text-center' : 'hidden px-6 py-16 text-center',
          data: { stock_check_target: 'emptyState' }
        ) do
          div(
            class: 'mx-auto mb-4 flex size-12 items-center justify-center rounded-shape-full ' \
                   'bg-secondary-container text-primary'
          ) { render Icons::Inventory.new(size: 24) }
          m3_heading(level: 3, variant: :title_medium, class: 'font-bold') do
            t('medications.stock_check.empty_title')
          end
          m3_text(variant: :body_medium, class: 'mt-1 text-on-surface-variant') do
            t('medications.stock_check.empty_description')
          end
        end
      end

      def render_batch_row(medication)
        selected = selected?(medication)
        div(
          hidden: !selected,
          class: selected ? batch_row_classes : "hidden #{batch_row_classes}",
          data: {
            stock_check_target: 'batchRow',
            medication_id: medication.id,
            current_supply: current_supply(medication).to_s('F'),
            increase_label: t('medications.stock_check.increase'),
            decrease_label: t('medications.stock_check.decrease'),
            no_change_label: t('medications.stock_check.no_change'),
            out_of_stock_label: t('medications.stock_check.out_of_stock')
          }
        ) do
          render_batch_identity(medication)
          div(class: 'text-sm font-medium text-on-surface-variant xl:text-on-surface') do
            span(class: 'xl:hidden') { "#{t('medications.stock_check.current')}: " }
            plain supply_label(medication)
          end
          render_quantity_field(medication, selected)
          render_difference(medication)
          render_batch_row_actions(medication)
        end
      end

      def batch_row_classes
        'grid gap-4 px-4 py-8 sm:grid-cols-[minmax(0,1fr)_auto] ' \
          'xl:grid-cols-[minmax(8rem,1fr)_4.5rem_minmax(9rem,1fr)_5.5rem_8rem] ' \
          'xl:items-center xl:gap-3'
      end

      def render_batch_identity(medication)
        div(class: 'flex min-w-0 items-center gap-3') do
          div(
            class: 'flex size-10 shrink-0 items-center justify-center rounded-shape-full ' \
                   'bg-secondary-container text-primary'
          ) { render Components::Shared::MedicationIcon.new(medication: medication, size: 19) }
          div(class: 'min-w-0') do
            div(class: 'truncate font-bold') { medication.display_name }
            m3_text(variant: :body_small, class: 'text-on-surface-variant') { dosage_label(medication) }
          end
        end
      end

      def render_quantity_field(medication, selected)
        id = quantity_id(medication)
        FormField(class: 'space-y-2 sm:col-span-2 xl:col-span-1') do
          FormFieldLabel(for: id, class: 'text-xs xl:sr-only') do
            t('medications.stock_check.new_supply_for', medication: medication.display_name)
          end
          div(class: 'flex items-stretch rounded-shape-sm bg-primary-container/20') do
            m3_input(
              id: id,
              type: :number,
              name: "stock_check[adjustments][#{medication.id}]",
              value: initial_adjustments[medication.id.to_s],
              disabled: !selected,
              required: true,
              min: 0,
              step: '0.01',
              class: 'h-12 min-h-12 rounded-r-none bg-card px-3 py-2 font-bold',
              data: {
                stock_check_target: 'quantity',
                medication_id: medication.id,
                action: 'input->stock-check#updateQuantity'
              }
            )
            span(
              class: 'flex items-center rounded-r-shape-sm border border-l-0 border-outline bg-card px-3 text-sm'
            ) { t('medications.stock_check.units') }
          end
        end
      end

      def render_difference(medication)
        div(
          class: 'text-sm font-bold text-on-surface-variant sm:text-right xl:text-left',
          data: { stock_check_target: 'difference', medication_id: medication.id },
          aria_live: 'polite'
        ) { initial_difference_label(medication) }
      end

      def render_batch_row_actions(medication)
        div(class: 'flex items-center justify-end gap-1 sm:col-span-2 xl:col-span-1') do
          m3_button(
            type: :button,
            variant: :text,
            size: :sm,
            class: 'px-2',
            data: {
              action: 'stock-check#setToZero',
              stock_check_id_param: medication.id
            }
          ) { t('medications.stock_check.set_to_zero') }
          m3_button(
            type: :button,
            variant: :outlined,
            size: :sm,
            class: 'size-10 p-0 text-on-surface-variant',
            aria_label: t('medications.stock_check.remove_from_batch', medication: medication.display_name),
            data: {
              action: 'stock-check#removeFromBatch',
              stock_check_id_param: medication.id
            }
          ) { render Icons::Trash.new(size: 16) }
        end
      end

      def render_reason_field
        FormField do
          FormFieldLabel(for: 'stock_check_reason') { t('medications.stock_check.reason') }
          m3_select(id: 'stock_check_reason', name: 'stock_check[reason]', size: :sm, class: 'h-12 min-h-12 bg-card') do
            reason_options.each do |option_value|
              option(value: option_value, selected: selected_reason == option_value) { option_value }
            end
          end
        end
      end

      def reason_options
        %w[house_stock_check manual_correction damaged_or_disposed other].map do |key|
          t("medications.stock_check.reasons.#{key}")
        end
      end

      def selected_reason
        reason.presence || reason_options.first
      end

      def render_batch_actions
        div(
          class: 'sticky bottom-0 mt-auto border-t border-border bg-surface-container-lowest/95 ' \
                 'px-4 py-5 backdrop-blur md:px-8'
        ) do
          div(class: 'flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between') do
            div(class: 'font-medium text-on-surface-variant', data: { stock_check_target: 'summary' },
                aria_live: 'polite') do
              render_initial_summary
            end
            div(class: 'flex flex-wrap justify-end gap-3') do
              m3_button(type: :button, variant: :outlined, data: { action: 'stock-check#clearSelection' }) do
                t('medications.stock_check.clear_batch')
              end
              m3_button(
                type: :submit,
                variant: :filled,
                disabled: initial_adjustments.empty?,
                data: { stock_check_target: 'submitButton' }
              ) do
                span(data: { stock_check_target: 'submitLabel' }) do
                  t('medications.stock_check.apply', count: initial_adjustments.size)
                end
              end
            end
          end
        end
      end

      def render_initial_summary
        span(data: { stock_check_target: 'summaryCount' }) do
          t('medications.stock_check.medicines', count: initial_adjustments.size)
        end
        plain ' '
        span(class: 'mx-2', aria_hidden: 'true') { '·' }
        plain ' '
        span { t('medications.stock_check.net_change') }
        plain ' '
        strong(data: { stock_check_target: 'netChange' }) { initial_net_change }
      end

      def initial_difference_label(medication)
        difference = initial_difference(medication)
        return '' if difference.nil?
        return t('medications.stock_check.no_change') if difference.zero?

        formatted_difference(difference)
      end

      def initial_net_change
        differences = medications.filter_map { |medication| initial_difference(medication) }
        formatted_difference(differences.sum)
      end

      def initial_difference(medication)
        value = initial_adjustments[medication.id.to_s]
        return if value.blank?

        BigDecimal(value) - current_supply(medication)
      rescue ArgumentError
        nil
      end

      def formatted_difference(difference)
        quantity = MedicationStockQuantityFormatter.format(difference.abs)
        sign = if difference.positive?
                 '+'
               elsif difference.negative?
                 '-'
               else
                 ''
               end
        "#{sign}#{quantity} #{t('medications.stock_check.units')}"
      end

      def selected?(medication)
        initial_adjustments.key?(medication.id.to_s)
      end

      def selected_location_name
        locations.find { |location| location.id == current_location_id.to_i }&.name ||
          t('medications.stock_check.all_locations')
      end

      def current_supply(medication)
        BigDecimal((medication.current_supply || 0).to_s)
      end

      def supply_label(medication)
        "#{MedicationStockQuantityFormatter.format(current_supply(medication))} " \
          "#{t('medications.stock_check.units')}"
      end

      def dosage_label(medication)
        if medication.dose_amount.blank? || medication.dose_unit.blank?
          return t('medications.stock_check.dosage_not_set')
        end

        "#{MedicationStockQuantityFormatter.format(medication.dose_amount)} #{medication.dose_unit}"
      end

      def selection_id(medication) = "stock_check_medication_#{medication.id}"
      def quantity_id(medication) = "stock_check_quantity_#{medication.id}"
    end
  end
end
