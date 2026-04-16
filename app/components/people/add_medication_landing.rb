# frozen_string_literal: true

module Components
  module People
    # Landing modal asking whether to add a scheduled or as-needed medication
    class AddMedicationLanding < Components::Base
      include Phlex::Rails::Helpers::TurboFrameTag
      include RubyUI

      attr_reader :person, :can_schedule, :can_person_medication, :back_path, :medication_id

      def initialize(person:, can_schedule: true, can_person_medication: true, back_path: nil, medication_id: nil)
        @person = person
        @can_schedule = can_schedule
        @can_person_medication = can_person_medication
        @back_path = back_path
        @medication_id = medication_id
        super()
      end

      def view_template
        turbo_frame_tag 'modal' do
          Dialog(open: true) do
            DialogContent(size: :md) do
              DialogHeader do
                if back_path
                  a(
                    href: back_path,
                    data: { turbo_frame: 'modal' },
                    class: 'inline-flex items-center text-sm text-on-surface-variant hover:text-foreground ' \
                           'transition-colors mb-2 no-underline'
                  ) do
                    plain t('medication_workflow.back')
                  end
                end
                DialogTitle { t('people.add_medication.title') }
                DialogDescription { t('people.add_medication.subtitle') }
              end
              DialogMiddle do
                div(class: 'grid grid-cols-1 gap-3 py-2') do
                  if can_schedule
                    render_option(
                      href: new_person_schedule_path(person, medication_id: medication_id),
                      title: t('people.add_medication.scheduled_title'),
                      description: t('people.add_medication.scheduled_description'),
                      icon: Icons::Calendar
                    )
                  end
                  if can_person_medication
                    render_option(
                      href: new_person_person_medication_path(person, medication_id: medication_id),
                      title: t('people.add_medication.otc_title'),
                      description: t('people.add_medication.otc_description'),
                      icon: Icons::Pill
                    )
                  end
                end
              end
            end
          end
        end
      end

      private

      def render_option(href:, title:, description:, icon:)
        a(
          href: href,
          data: { turbo_frame: 'modal' },
          class: 'flex items-start gap-4 w-full rounded-2xl border-2 border-outline p-6 ' \
                 'hover:border-primary hover:bg-primary/5 active:bg-primary/10 ' \
                 'transition-all cursor-pointer no-underline'
        ) do
          div(class: 'w-12 h-12 rounded-xl bg-primary/10 flex items-center justify-center ' \
                     'text-primary flex-none mt-0.5') do
            render icon.new(size: 24)
          end
          div do
            div(class: 'font-semibold text-base text-foreground') { title }
            div(class: 'text-on-surface-variant text-sm mt-1') { description }
          end
        end
      end
    end
  end
end
