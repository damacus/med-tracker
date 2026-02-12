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
        CardHeader do
          render_person_icon
          div(class: 'flex items-center justify-between gap-2') do
            Heading(level: 2, size: '5', class: 'font-semibold leading-none tracking-tight') do
              Link(href: person_path(person), variant: :link, class: 'text-xl font-semibold') { person.name }
            end
            render_needs_carer_badge if person.needs_carer?
          end
        end
      end

      def render_person_icon
        div(class: 'w-10 h-10 rounded-xl flex items-center justify-center bg-violet-100 text-violet-700 mb-2') do
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
              strong { 'Prescriptions: ' }
              plain prescription_count_text
            end
          end
        end
      end

      def prescription_count_text
        if person.prescriptions.active.any?
          view_context.pluralize(person.prescriptions.active.count, 'active prescription')
        else
          'No active prescriptions'
        end
      end

      def render_needs_carer_badge
        render RubyUI::Badge.new(
          variant: :warning,
          size: :md,
          data: { testid: 'needs-carer-badge' }
        ) { 'Needs Carer' }
      end

      def render_card_footer
        CardFooter(class: 'flex gap-2 flex-wrap') do
          Link(
            href: new_person_prescription_path(person),
            variant: :primary,
            size: :md,
            data: { turbo_stream: true }
          ) { 'Add Prescription' }

          if person.prescriptions.any?
            Link(
              href: person_path(person),
              variant: :outline,
              size: :md,
              data: { turbo_stream: true }
            ) { 'View Prescriptions' }
          end

          render_assign_carer_link if person.needs_carer?
        end
      end

      def render_assign_carer_link
        Link(
          href: new_admin_carer_relationship_path(patient_id: person.id),
          variant: :outline,
          size: :md,
          class: 'text-amber-700 border-amber-300 hover:bg-amber-50'
        ) { 'Assign Carer' }
      end
    end
  end
end
