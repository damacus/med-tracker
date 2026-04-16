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
        m3_card(class: 'p-6') do
          div(class: 'flex items-center justify-between mb-4') do
            m3_heading(variant: :title_medium, level: 3, class: 'font-bold') { t('medications.show.dosages_heading') }
            if can_manage?
              m3_link(
                href: view_context.edit_medication_path(
                  medication,
                  return_to: view_context.medication_path(medication)
                ),
                variant: :outlined,
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
            m3_text(variant: :body_medium, class: 'text-on-surface-variant italic font-medium') do
              t('medications.show.no_dosages')
            end
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
              m3_link(
                href: view_context.edit_medication_path(
                  medication,
                  return_to: view_context.medication_path(medication)
                ),
                variant: :text,
                size: :sm
              ) { t('medications.show.edit_dosage') }
            end
          end
        end
      end

      def render_dosage_summary(dosage)
        div(class: 'flex items-center gap-2 flex-wrap') do
          span(class: 'font-semibold text-sm') { "#{dosage.amount.to_f} #{dosage.unit}" }
          span(class: 'text-on-surface-variant text-sm') { dosage.frequency }
          if dosage.default_for_adults?
            m3_badge(variant: :outlined, class: 'text-xs') { t('dosages.form.default_for_adults') }
          end
          m3_badge(variant: :tonal, class: 'text-xs') { t('medications.show.children') } if dosage.default_for_children?
        end
      end

      def render_dosage_scheduling_hint(dosage)
        return unless dosage.default_max_daily_doses || dosage.default_min_hours_between_doses

        div(class: 'text-xs text-on-surface-variant font-medium') do
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
