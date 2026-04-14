# frozen_string_literal: true

module Components
  module Medications
    class DoseHistoryComponent < Components::Base
      include Phlex::Rails::Helpers::TurboFrameTag

      attr_reader :medication

      def initialize(medication:)
        @medication = medication
        super()
      end

      def view_template
        Card(class: 'p-6') do
          div(class: 'flex items-center justify-between mb-4') do
            Heading(level: 3, size: '4', class: 'font-bold') { t('medications.show.dosages_heading') }
            if can_manage?
              Link(
                href: view_context.edit_medication_path(
                  medication,
                  return_to: view_context.medication_path(medication)
                ),
                variant: :outline,
                size: :sm
              ) { t('medications.show.add_dosage') }
            end
          end

          if dosages.any?
            div(class: 'space-y-3') do
              dosages.each do |dosage|
                render_dosage_row(dosage)
              end
            end
          else
            Text(size: '2', class: 'text-muted-foreground italic') { t('medications.show.no_dosages') }
          end
        end
      end

      private

      def dosages
        @dosages ||= medication.dosages.sort_by(&:amount)
      end

      def can_manage?
        @can_manage ||= begin
          view_context.policy(medication).update?
        rescue StandardError
          false
        end
      end

      def render_dosage_row(dosage)
        div(class: 'flex items-start justify-between gap-3 rounded-shape-lg border border-border p-3') do
          div(class: 'space-y-1') do
            render_dosage_summary(dosage)
            render_dosage_scheduling_hint(dosage)
          end

          if can_manage?
            div(class: 'flex gap-2 flex-none') do
              Link(
                href: view_context.edit_medication_path(
                  medication,
                  return_to: view_context.medication_path(medication)
                ),
                variant: :ghost,
                size: :sm
              ) { t('medications.show.edit_dosage') }
            end
          end
        end
      end

      def render_dosage_summary(dosage)
        div(class: 'flex items-center gap-2 flex-wrap') do
          span(class: 'font-semibold text-sm') { "#{dosage.amount.to_f} #{dosage.unit}" }
          span(class: 'text-muted-foreground text-sm') { dosage.frequency }
          if dosage.default_for_adults?
            Badge(variant: :outline, class: 'text-xs') { t('dosages.form.default_for_adults') }
          end
          if dosage.default_for_children?
            Badge(variant: :secondary, class: 'text-xs') { t('medications.show.children') }
          end
        end
      end

      def render_dosage_scheduling_hint(dosage)
        return unless dosage.default_max_daily_doses || dosage.default_min_hours_between_doses

        div(class: 'text-xs text-muted-foreground') do
          parts = []
          if dosage.default_max_daily_doses
            parts << t('medications.show.max_per_cycle', count: dosage.default_max_daily_doses)
          end
          if dosage.default_min_hours_between_doses
            parts << t('medications.show.min_hours_apart', hours: dosage.default_min_hours_between_doses)
          end
          plain parts.join(' · ')
        end
      end
    end
  end
end
