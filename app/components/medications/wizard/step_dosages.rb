# frozen_string_literal: true

module Components
  module Medications
    module Wizard
      class StepDosages < Components::Base
        include Phlex::Rails::Helpers::TurboFrameTag

        attr_reader :medication

        def initialize(medication:)
          @medication = medication
          super()
        end

        def view_template
          div(id: 'wizard-content', class: 'space-y-8') do
            render_header
            render_dosage_list
            render_dosage_form
            render_actions
          end
        end

        private

        def render_header
          div(class: 'text-center space-y-3') do
            div(
              class: 'mx-auto w-14 h-14 rounded-2xl bg-emerald-50 flex items-center justify-center ' \
                     'text-emerald-600 mb-2'
            ) do
              render Icons::CheckCircle.new(size: 28)
            end
            Heading(level: 3, size: '5', class: 'font-bold tracking-tight text-slate-900') do
              "#{medication.name} created!"
            end
            Text(size: '2', class: 'text-slate-400 max-w-sm mx-auto') do
              'Now add dose size options. For example, different amounts for adults and children.'
            end
          end
        end

        def render_dosage_list
          div(id: 'dosage-list', class: 'space-y-3') do
            medication.dosages.reload.order(:amount).each do |dosage|
              render DosageRow.new(dosage: dosage)
            end
          end
        end

        def render_dosage_form
          turbo_frame_tag 'dosage-form' do
            div(class: 'rounded-2xl border border-dashed border-slate-200 p-6 bg-slate-50/50') do
              render WizardDosageForm.new(
                dosage: medication.dosages.build,
                medication: medication
              )
            end
          end
        end

        def render_actions
          div(class: 'flex items-center justify-between pt-6 border-t border-slate-100') do
            Link(
              href: medication_path(medication),
              variant: :ghost,
              class: 'font-bold text-slate-400 hover:text-slate-600',
              data: { turbo_frame: '_top' }
            ) { 'Skip for now' }

            Link(
              href: medication_path(medication),
              variant: :primary,
              size: :lg,
              class: 'px-8 rounded-2xl shadow-lg shadow-primary/20',
              data: { turbo_frame: '_top' }
            ) { 'Done' }
          end
        end
      end
    end
  end
end
