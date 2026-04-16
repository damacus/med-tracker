# frozen_string_literal: true

module Components
  module Medications
    class StandardDosageComponent < Components::Base
      attr_reader :medication

      def initialize(medication:)
        @medication = medication
        super()
      end

      def view_template
        m3_card(class: 'p-8 space-y-6') do
          m3_heading(variant: :title_medium, level: 3, class: 'font-bold') { t('medications.show.standard_dosage') }

          if dosage_specified?
            div(class: 'flex items-center gap-4') do
              div(
                class: 'w-12 h-12 rounded-shape-xl bg-success/10 flex items-center justify-center ' \
                       'text-success shadow-inner'
              ) do
                render Icons::CheckCircle.new(size: 24)
              end
              div do
                span(class: 'text-3xl font-black text-foreground tracking-tight') { medication.dosage_amount.to_s }
                span(class: 'text-lg font-bold text-on-surface-variant ml-1') { medication.dosage_unit }
              end
            end
          else
            m3_text(variant: :body_medium, class: 'text-on-surface-variant italic font-medium') do
              t('medications.show.no_dosage')
            end
          end

          div(class: 'pt-4 border-t border-border') do
            render_overview_item(
              t('medications.show.reorder_at_label'),
              pluralize(medication.reorder_threshold, 'unit')
            )
          end
        end
      end

      private

      def dosage_specified?
        medication.dosage_amount.present? && medication.dosage_unit.present?
      end

      def render_overview_item(label, value)
        div(class: 'flex items-center gap-4 group') do
          div(
            class: 'w-10 h-10 rounded-shape-xl bg-primary/10 flex items-center justify-center ' \
                   'text-primary ' \
                   'group-hover:bg-primary/20 transition-colors shadow-inner'
          ) do
            render Icons::Settings.new(size: 20)
          end
          div do
            m3_text(variant: :label_small, class: 'uppercase tracking-widest text-on-surface-variant font-black') do
              label
            end
            m3_text(variant: :body_medium, class: 'font-semibold') { value }
          end
        end
      end
    end
  end
end