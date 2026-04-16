# frozen_string_literal: true

module Components
  module Medications
    module Wizard
      class StepDosages < Components::Base
        attr_reader :medication

        def initialize(medication:)
          @medication = medication
          super()
        end

        def view_template
          div(id: 'wizard-content', class: 'space-y-8') do
            render_header
            render_dosage_list
            render_actions
          end
        end

        private

        def render_header
          div(class: 'text-center space-y-3') do
            div(
              class: 'mx-auto w-14 h-14 rounded-shape-xl bg-success-container flex items-center justify-center ' \
                     'text-on-success-container mb-2'
            ) do
              render Icons::CheckCircle.new(size: 28)
            end
            m3_heading(level: 3, size: '5', class: 'font-bold tracking-tight text-foreground') do
              "#{medication.name} created!"
            end
            m3_text(size: '2', class: 'text-on-surface-variant max-w-sm mx-auto') do
              'Review the dose options and continue in the medication editor if you need to add or adjust them.'
            end
          end
        end

        def render_dosage_list
          div(id: 'dosage-list', class: 'space-y-3') do
            if medication.dosages.any?
              medication.dosages.sort_by(&:amount).each do |dosage|
                render DosageRow.new(dosage: dosage)
              end
            else
              div(
                class: 'rounded-shape-xl border border-dashed border-border p-6 bg-card text-center'
              ) do
                m3_text(size: '2', class: 'text-on-surface-variant') do
                  'No dose options are configured yet.'
                end
              end
            end
          end
        end

        def render_actions
          div(class: 'flex items-center justify-between pt-6 border-t border-border') do
            Link(
              href: edit_medication_path(medication, return_to: medication_path(medication)),
              variant: :outlined,
              class: 'font-bold',
              data: { turbo_frame: '_top' }
            ) { 'Manage dose options' }

            Link(
              href: medication_path(medication),
              variant: :filled,
              size: :lg,
              class: 'px-8 rounded-shape-xl shadow-lg shadow-primary/20',
              data: { turbo_frame: '_top' }
            ) { 'Done' }
          end
        end
      end
    end
  end
end