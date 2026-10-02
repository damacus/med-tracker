module Components
  module Medications
    class StockRemovalView < Components::Base
      include Phlex::Rails::Helpers::L

      def initialize(medication:, dosages:, removals:, attributes:, error: nil)
        @medication = medication
        @dosages = dosages
        @removals = removals
        @attributes = attributes
        @error = error
        super()
      end

      def view_template
        div(class: 'mx-auto max-w-2xl space-y-6 px-4 py-8 pb-28') do
          m3_link(href: medication_path(medication), variant: :text) { medication.display_name }
          m3_heading(level: 1) { t('stock_removals.title') }
          m3_text { t('stock_removals.description') }
          Card(class: 'p-6 space-y-5') do
            render_error
            render_form
          end
          render_history
        end
      end

      private

      attr_reader :medication, :dosages, :removals, :attributes, :error

      def render_error
        return unless error

        div(id: 'stock-removal-error', role: 'alert', class: 'text-error font-semibold', tabindex: '-1') do
          t("stock_removals.errors.#{error}")
        end
      end

      def render_form
        form_with(url: medication_stock_removals_path(medication), method: :post,
                  class: 'space-y-5', data: { turbo_frame: '_top' }) do
          input(type: 'hidden', name: 'stock_removal[submission_id]', value: attributes[:submission_id])
          render_source
          FormField do
            FormFieldLabel(for: 'removal_quantity') { t('stock_removals.quantity') }
            m3_input(id: 'removal_quantity', name: 'stock_removal[quantity]', type: :number, min: '0.01', step: '0.01',
                     required: true, value: attributes[:quantity],
                     aria: { describedby: error ? 'stock-removal-error' : nil })
          end
          render_reason
          FormField do
            FormFieldLabel(for: 'removal_note') { t('stock_removals.note') }
            Textarea(id: 'removal_note', name: 'stock_removal[note]', maxlength: 1000, rows: 3,
                     class: 'w-full rounded-xl border border-outline bg-surface p-3 focus:ring-2 focus:ring-primary') do
              attributes[:note]
            end
          end
          div(class: 'flex flex-wrap gap-3') do
            m3_button(type: :submit, variant: :filled) { t('stock_removals.submit') }
            m3_link(href: medication_path(medication), variant: :text) { t('stock_removals.cancel') }
          end
        end
      end

      def render_source
        if dosages.empty?
          m3_text do
            t('stock_removals.available', quantity: formatted(medication.current_supply || 0),
                                          unit: stock_unit(medication.dose_unit))
          end
        else
          FormField do
            FormFieldLabel(for: 'removal_source') { t('stock_removals.source') }
            m3_select(id: 'removal_source', name: 'stock_removal[dosage_id]', required: true) do
              option(value: '') { t('stock_removals.choose_source') }
              dosages.each do |dosage|
                option(value: dosage.id, selected: attributes[:dosage_id].to_s == dosage.id.to_s) do
                  dosage_label(dosage)
                end
              end
            end
          end
        end
      end

      def render_reason
        FormField do
          FormFieldLabel(for: 'removal_reason') { t('stock_removals.reason') }
          m3_select(id: 'removal_reason', name: 'stock_removal[reason]', required: true) do
            option(value: '') { t('stock_removals.choose_reason') }
            RemoveMedicationStockService::REASONS.each do |reason|
              option(value: reason, selected: attributes[:reason] == reason) { t("stock_removals.reasons.#{reason}") }
            end
          end
        end
      end

      def dosage_label(dosage)
        available = t('stock_removals.available', quantity: formatted(dosage.current_supply),
                                                  unit: stock_unit(dosage.unit))
        "#{dosage.amount} #{dosage.unit} — #{available}"
      end

      def render_history
        section(class: 'space-y-4') do
          m3_heading(level: 2) { t('stock_removals.history') }
          m3_text { t('stock_removals.empty') } if removals.empty?
          removals.each do |removal|
            Card(class: 'p-4 space-y-2') do
              m3_text do
                t('stock_removals.history_summary', quantity: removal['quantity'], unit: stock_unit(removal['unit']),
                                                    reason: t("stock_removals.reasons.#{removal['reason']}"))
              end
              m3_text do
                t('stock_removals.available', quantity: removal['remaining_quantity'],
                                              unit: stock_unit(removal['unit']))
              end
              m3_text { removal['note'] } if removal['note'].present?
              m3_text do
                "#{l(removal['at'], format: :short)} · #{removal['actor'] || t('stock_removals.unknown_actor')}"
              end
            end
          end
        end
      end

      def formatted(quantity)
        MedicationStockQuantityFormatter.format(quantity)
      end

      def stock_unit(unit)
        return t('stock_removals.units') unless MedicationStockConsumption.stock_quantity_unit?(unit)

        unit
      end
    end
  end
end
