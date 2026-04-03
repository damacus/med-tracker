# frozen_string_literal: true

module Components
  module Medications
    module Wizard
      class DosageRow < Components::Base
        attr_reader :dosage

        def initialize(dosage:)
          @dosage = dosage
          super()
        end

        def view_template
          div(
            id: "wizard_dosage_#{dosage.id}",
            class: 'flex items-center justify-between rounded-xl border border-border bg-surface-container-lowest p-4 shadow-sm'
          ) do
            div(class: 'flex items-center gap-3') do
              div(
                class: 'w-10 h-10 rounded-xl bg-primary/5 flex items-center justify-center text-primary'
              ) do
                render Icons::Pill.new(size: 18)
              end
              div do
                span(class: 'font-bold text-sm text-foreground') do
                  "#{dosage.amount.to_f} #{dosage.unit}"
                end
                span(class: 'text-xs text-muted-foreground ml-2') { dosage.frequency } if dosage.frequency.present?
                render_default_badges
              end
            end
          end
        end

        private

        def render_default_badges
          return unless dosage.default_for_adults? || dosage.default_for_children?

          div(class: 'flex gap-1 mt-1') do
            Badge(variant: :outline, class: 'text-[10px]') { 'Adults' } if dosage.default_for_adults?
            Badge(variant: :secondary, class: 'text-[10px]') { 'Children' } if dosage.default_for_children?
          end
        end
      end
    end
  end
end
