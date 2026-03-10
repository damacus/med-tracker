# frozen_string_literal: true

module Components
  module People
    # Person card component
    class PersonCard < Components::Base
      attr_reader :person

      def initialize(person:)
        @person = person
        super()
      end

      def view_template
        Card(id: "person_#{person.id}", class: 'h-full flex flex-col') do
          render_card_header
          render_card_content
          render_card_footer
        end
      end

      private

      def render_card_header
        CardHeader(class: 'pb-4') do
          div(class: 'flex items-start justify-between') do
            render_person_icon
            render_needs_carer_badge if person.needs_carer?
          end
          div(class: 'space-y-1') do
            Heading(level: 2, size: '5', class: 'font-semibold leading-none tracking-tight') do
              Link(
                href: person_path(person),
                variant: :link,
                class: 'text-2xl font-bold p-0 h-auto hover:no-underline'
              ) { person.name }
            end
          end
        end
      end

      def render_person_icon
        div(class: 'w-10 h-10 rounded-xl flex items-center justify-center bg-violet-100 text-violet-700') do
          render Icons::User.new(size: 20)
        end
      end

      def render_card_content
        CardContent(class: 'flex-grow space-y-2') do
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
              strong { 'Medications: ' }
              plain active_medication_count_text
            end
          end
        end
      end

      def active_schedules
        @active_schedules ||= if person.schedules.loaded?
                                person.schedules.select(&:active?)
                              else
                                person.schedules.active
                              end
      end

      def visible_person_medications
        @visible_person_medications ||= person.person_medications.select do |person_medication|
          view_context.policy(person_medication).show?
        end
      end

      def active_medications_count
        active_schedules.size + visible_person_medications.size
      end

      def active_medication_count_text
        if active_medications_count.positive?
          view_context.pluralize(active_medications_count, 'active medication')
        else
          'No active medications'
        end
      end

      def render_needs_carer_badge
        render RubyUI::Badge.new(
          variant: :warning,
          size: :sm,
          class: 'h-auto py-1.5 px-3 rounded-xl border-amber-200 bg-amber-50 text-amber-900 ' \
                 'font-bold uppercase tracking-wider text-[10px]',
          data: { testid: 'needs-carer-badge' }
        ) do
          div(class: 'flex flex-col items-center leading-tight') do
            plain 'Needs Carer'
          end
        end
      end

      def render_card_footer
        CardFooter(class: 'flex flex-col gap-3 mt-auto pt-6') do
          render_add_medication_link if can_add_medication?

          div(class: 'flex gap-2 w-full') do
            if active_medications_count.positive?
              Link(
                href: person_path(person),
                variant: :outline,
                size: :md,
                class: 'flex-1 rounded-2xl'
              ) { 'View Medications' }
            end

            render_assign_carer_link if person.needs_carer? && can_create?(CarerRelationship)
          end
        end
      end

      def render_add_medication_link
        Link(
          href: add_medication_person_path(person),
          variant: :primary,
          size: :md,
          class: 'w-full rounded-2xl font-bold py-6'
        ) { 'Add Medication' }
      end

      def can_add_medication?
        can_create?(Schedule.new(person: person)) ||
          can_create?(PersonMedication.new(person: person))
      end

      def render_assign_carer_link
        Link(
          href: new_admin_carer_relationship_path(patient_id: person.id),
          variant: :outline,
          size: :md,
          class: 'flex-1 rounded-2xl text-amber-700 border-amber-300 hover:bg-amber-50',
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
