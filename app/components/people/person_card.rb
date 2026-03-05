# frozen_string_literal: true

module Components
  module People
    # Person card component
    class PersonCard < Components::Base
      attr_reader :person, :current_user

      def initialize(person:, current_user:)
        @person = person
        @current_user = current_user
        super()
      end

      def view_template
        render RubyUI::Card.new(id: "person_#{person.id}", class: 'h-full flex flex-col') do
          render_card_header
          render_card_content
          render_card_footer
        end
      end

      private

      def render_card_header
        render RubyUI::CardHeader.new do
          div(class: 'w-10 h-10 rounded-xl flex items-center justify-center bg-violet-100 text-violet-700 mb-2') do
            render Icons::User.new(size: 20)
          end
          div(class: 'flex items-center justify-between gap-2') do
            render RubyUI::Heading.new(level: 2, size: '5', class: 'font-semibold leading-none tracking-tight') do
              render RubyUI::Link.new(href: view_context.person_path(person), variant: :link, class: 'text-xl font-semibold p-0 h-auto') { person.name }
            end
            render_needs_carer_badge if person.needs_carer?
          end
        end
      end

      def render_card_content
        render RubyUI::CardContent.new(class: 'flex-grow space-y-2') do
          div(class: 'space-y-1 text-sm text-muted-foreground') do
            p do
              strong { 'Born: ' }
              plain person.date_of_birth.strftime('%B %d, %Y')
            end
            p do
              strong { 'Age: ' }
              plain person.age.to_s
            end
            p do
              strong { 'Schedules: ' }
              plain schedule_count_text
            end
          end
        end
      end

      def render_card_footer
        render RubyUI::CardFooter.new(class: 'flex gap-2 flex-wrap border-t pt-4 mt-auto') do
          render_add_medication_link if can_add_medication?

          if person.schedules.any?
            render RubyUI::Link.new(
              href: view_context.person_path(person),
              variant: :outline,
              size: :md
            ) { 'View Schedules' }
          end

          render_assign_carer_link if person.needs_carer? && can_create?(CarerRelationship)
        end
      end

      def active_schedules
        @active_schedules ||= if person.schedules.loaded?
                                person.schedules.select(&:active?)
                              else
                                person.schedules.active
                              end
      end

      def schedule_count_text
        if active_schedules.present?
          view_context.pluralize(active_schedules.size, 'active schedule')
        else
          'No active schedules'
        end
      end

      def render_needs_carer_badge
        render RubyUI::Badge.new(
          variant: :warning,
          size: :md,
          data: { testid: 'needs-carer-badge' }
        ) { 'Needs Carer' }
      end

      def render_add_medication_link
        render RubyUI::Link.new(
          href: view_context.add_medication_person_path(person),
          variant: :primary,
          size: :md,
          data: { turbo_frame: 'modal' }
        ) { 'Add Medication' }
      end

      def can_add_medication?
        can_create?(Schedule.new(person: person)) ||
          can_create?(PersonMedication.new(person: person))
      end

      def render_assign_carer_link
        render RubyUI::Link.new(
          href: view_context.new_admin_carer_relationship_path(patient_id: person.id),
          variant: :outline,
          size: :md,
          class: 'text-amber-700 border-amber-300 hover:bg-amber-50',
          data: { turbo_frame: 'modal' }
        ) { 'Assign Carer' }
      end

      def can_create?(record)
        return false unless view_context.respond_to?(:policy)

        view_context.policy(record).create?
      end
    end
  end
end
