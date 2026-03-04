module Components
  module People
    class PersonCard < RubyUI::Card
      attr_reader :person, :current_user

      def initialize(person:, current_user:)
        @person = person
        @current_user = current_user
      end

      def view_template
        render RubyUI::CardHeader.new do
          div(class: 'flex items-center justify-between gap-4') do
            div(class: 'flex items-center gap-4') do
              render People::Avatar.new(person: person, size: :lg)
              div do
                h3(class: 'font-semibold leading-none tracking-tight text-lg') { person.name }
                p(class: 'text-sm text-muted-foreground mt-1') { person_details_text }
              end
            end
            render People::StatusBadge.new(person: person)
          end
        end

        render RubyUI::CardContent.new do
          div(class: 'grid gap-4') do
            div(class: 'flex items-center justify-between text-sm') do
              span(class: 'text-muted-foreground') { 'Active Schedules' }
              span(class: 'font-medium') { schedule_count_text }
            end
            
            if active_schedules.any?
              div(class: 'space-y-2') do
                active_schedules.take(3).each do |schedule|
                  div(class: 'flex items-center justify-between text-xs p-2 rounded-md bg-muted/50') do
                    span(class: 'truncate font-medium') { schedule.medication_name }
                    span(class: 'text-muted-foreground shrink-0 ml-2') { schedule.dosage_text }
                  end
                end
                if active_schedules.size > 3
                  p(class: 'text-[10px] text-center text-muted-foreground') do
                    "+ #{active_schedules.size - 3} more schedules"
                  end
                end
              end
            end
          end
        end

        render RubyUI::CardFooter.new(class: 'flex justify-between gap-2 border-t pt-4 mt-auto') do
          a(href: view_context.person_path(person), class: 'flex-1') do
            render RubyUI::Button.new(variant: :outline, class: 'w-full') { 'View Profile' }
          end
          if Pundit.policy(current_user, person).edit?
            a(href: view_context.edit_person_path(person), class: 'flex-1') do
              render RubyUI::Button.new(variant: :secondary, class: 'w-full') { 'Edit' }
            end
          end
        end
      end

      private

      def person_details_text
        parts = [person.person_type.titleize]
        parts << "#{person.age} years old" if person.age.present?
        parts.join(' • ')
      end

      def active_schedules
        @active_schedules ||= if person.respond_to?(:schedules)
                                if person.schedules.loaded?
                                  person.schedules.select(&:active?)
                                else
                                  person.schedules.active
                                end
                              else
                                []
                              end
      end

      def schedule_count_text
        return 'No active schedules' if active_schedules.empty?
        view_context.pluralize(active_schedules.size, 'active schedule')
      end
    end
  end
end
