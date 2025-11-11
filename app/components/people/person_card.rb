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
            CardTitle(class: 'text-xl') { person.name }
            render_needs_carer_badge if person.needs_carer?
          end
        end
      end

      def render_person_icon
        div(class: 'w-10 h-10 rounded-xl flex items-center justify-center bg-violet-100 text-violet-700 mb-2') do
          svg(
            xmlns: 'http://www.w3.org/2000/svg',
            width: '20',
            height: '20',
            viewBox: '0 0 24 24',
            fill: 'none',
            stroke: 'currentColor',
            stroke_width: '2',
            stroke_linecap: 'round',
            stroke_linejoin: 'round'
          ) do |s|
            s.path(d: 'M19 21v-2a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v2')
            s.circle(cx: '12', cy: '7', r: '4')
          end
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
        badge_classes = 'inline-flex items-center rounded-md bg-amber-50 px-2 py-1 text-xs ' \
                        'font-medium text-amber-800 ring-1 ring-inset ring-amber-600/20'
        span(
          class: badge_classes,
          data: { testid: 'needs-carer-badge' }
        ) do
          plain 'Needs Carer'
        end
      end

      def render_card_footer
        CardFooter(class: 'flex gap-2 flex-wrap') do
          Link(
            href: new_person_prescription_path(person),
            variant: :link,
            size: :sm,
            data: { turbo_stream: true }
          ) { 'Add Prescription' }

          if person.prescriptions.any?
            Link(
              href: person_path(person),
              variant: :link,
              size: :sm,
              data: { turbo_stream: true }
            ) { 'View Prescriptions' }
          end
        end
      end

      def helpers
        @helpers ||= ApplicationController.helpers
      end
    end
  end
end
