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
        Card(class: 'p-8 space-y-6') do
          Heading(level: 3, size: '4', class: 'font-bold') { t('medications.show.standard_dosage') }

          if dosage_specified?
            div(class: 'flex items-center gap-4') do
              div(
                class: 'w-12 h-12 rounded-2xl bg-secondary-container flex items-center justify-center ' \
                       'text-on-secondary-container shadow-sm'
              ) do
                render Icons::CheckCircle.new(size: 24)
              end
              div do
                span(class: 'text-3xl font-black text-foreground') { medication.dosage_amount.to_s }
                span(class: 'text-lg font-bold text-muted-foreground ml-1') { medication.dosage_unit }
              end
            end
          else
            Text(size: '2', class: 'text-muted-foreground italic') { t('medications.show.no_dosage') }
          end

          div(class: 'pt-4 border-t border-surface-container-low') do
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
            class: 'w-10 h-10 rounded-xl bg-surface-container-low flex items-center justify-center ' \
                   'text-muted-foreground ' \
                   'group-hover:bg-primary/5 group-hover:text-primary transition-colors'
          ) do
            render Icons::Settings.new(size: 20)
          end
          div do
            Text(size: '1', weight: 'black', class: 'uppercase tracking-widest text-muted-foreground') { label }
            Text(size: '2', weight: 'semibold') { value }
          end
        end
      end
    end
  end
end
