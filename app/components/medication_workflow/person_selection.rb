# frozen_string_literal: true

module Components
  module MedicationWorkflow
    # Step 1 of the global medication workflow: select which person to add a medication for
    class PersonSelection < Components::Base
      include Phlex::Rails::Helpers::TurboFrameTag
      include RubyUI

      attr_reader :people, :medication_id, :return_to

      def initialize(people:, medication_id: nil, return_to: nil)
        @people = people
        @medication_id = medication_id
        @return_to = return_to
        super()
      end

      def view_template
        turbo_frame_tag 'modal' do
          Dialog(open: true) do
            DialogContent(size: :md) do
              DialogHeader do
                DialogTitle { t('medication_workflow.person_selection.title') }
                DialogDescription { t('medication_workflow.person_selection.subtitle') }
              end
              DialogMiddle do
                if people.size <= 4
                  render_person_buttons
                else
                  render_person_combobox
                end
              end
            end
          end
        end
      end

      private

      def render_person_buttons
        div(class: 'grid grid-cols-1 gap-3 py-2') do
          people.each do |person|
            m3_link(
              href: person_assignment_path(person),
              variant: :outlined,
              size: :lg,
              data: { turbo_frame: 'modal' },
              class: 'w-full justify-start gap-4 rounded-2xl border-2 border-outline p-4 ' \
                     'hover:border-primary hover:bg-primary/5 active:bg-primary/10'
            ) do
              render Components::Shared::PersonAvatar.new(person: person, size: :md)
              div do
                div(class: 'font-semibold text-sm text-foreground') { person.name }
                div(class: 'text-on-surface-variant text-xs mt-0.5') { person.person_type.humanize }
              end
            end
          end
        end
      end

      def render_person_combobox
        div(class: 'py-2', data: { controller: 'medication-workflow' }) do
          search_placeholder = t('medication_workflow.person_selection.search_placeholder')
          render RubyUI::Combobox.new(class: 'w-full') do
            render RubyUI::ComboboxTrigger.new(placeholder: search_placeholder)

            render RubyUI::ComboboxPopover.new do
              render RubyUI::ComboboxSearchInput.new(placeholder: search_placeholder)

              render RubyUI::ComboboxList.new do
                render(RubyUI::ComboboxEmptyState.new { t('medication_workflow.person_selection.no_results') })

                people.each do |person|
                  render RubyUI::ComboboxItem.new do
                    render Components::Shared::PersonAvatar.new(person: person, size: :sm)
                    render RubyUI::ComboboxRadio.new(
                      name: 'person_id',
                      id: "person_#{person.id}",
                      value: person.id,
                      data: {
                        text: person.name,
                        action: 'change->medication-workflow#navigateToType',
                        url: person_assignment_path(person)
                      }
                    )
                    span { person.name }
                  end
                end
              end
            end
          end
        end
      end

      def person_assignment_path(person)
        new_person_medication_assignment_path(
          person,
          source: :workflow,
          medication_id: medication_id,
          return_to: return_to
        )
      end
    end
  end
end
