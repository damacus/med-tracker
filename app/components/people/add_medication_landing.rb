# frozen_string_literal: true

module Components
  module People
    # Landing modal asking whether to add a scheduled or as-needed medication
    class AddMedicationLanding < Components::Base
      include Phlex::Rails::Helpers::TurboFrameTag
      include RubyUI

      attr_reader :person, :can_schedule, :can_person_medication

      def initialize(person:, can_schedule: true, can_person_medication: true)
        @person = person
        @can_schedule = can_schedule
        @can_person_medication = can_person_medication
        super()
      end

      def view_template
        turbo_frame_tag 'modal' do
          Dialog(open: true) do
            DialogContent(size: :md) do
              DialogHeader do
                DialogTitle { t('people.add_medication.title') }
                DialogDescription { t('people.add_medication.subtitle') }
              end
              DialogMiddle do
                div(class: 'grid grid-cols-1 gap-3 py-2') do
                  if can_schedule
                    render_option(
                      href: new_person_schedule_path(person),
                      title: t('people.add_medication.scheduled_title'),
                      description: t('people.add_medication.scheduled_description'),
                      icon: Icons::Calendar
                    )
                  end
                  if can_person_medication
                    render_option(
                      href: new_person_person_medication_path(person),
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
          class: 'flex items-start gap-4 rounded-xl border border-input p-5 ' \
                 'hover:bg-accent hover:border-primary/30 transition-colors cursor-pointer no-underline'
        ) do
          div(class: 'w-10 h-10 rounded-xl bg-primary/10 flex items-center justify-center text-primary flex-none mt-0.5') do
            render icon.new(size: 20)
          end
          div do
            div(class: 'font-semibold text-sm text-foreground') { title }
            div(class: 'text-muted-foreground text-sm mt-0.5') { description }
          end
        end
      end
    end
  end
end
