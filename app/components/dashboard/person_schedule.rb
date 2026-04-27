# frozen_string_literal: true

module Components
  module Dashboard
    # Renders a person's medication schedule within the dashboard
    class PersonSchedule < Components::Base
      include Phlex::Rails::Helpers::ButtonTo
      include Pundit::Authorization

      attr_reader :person, :schedules, :current_user

      def initialize(person:, schedules:, current_user: nil)
        @person = person
        @schedules = schedules
        @current_user = current_user
        super()
      end

      def view_template
        div(class: 'space-y-4') do
          render_person_header
          render_schedules_grid
        end
      end

      private

      def render_person_header
        div(class: 'flex items-center gap-3 mb-2') do
          render_person_avatar
          div do
            m3_heading(level: 3) { person.name }
            m3_text(size: '2', weight: 'muted') { "#{t('dashboard.person_schedule.age')}: #{person.age}" }
          end
        end
      end

      def render_person_avatar
        div(class: 'w-12 h-12 rounded-full flex items-center justify-center bg-surface-container text-foreground') do
          render Icons::User.new(size: 24)
        end
      end

      def render_schedules_grid
        div(class: 'grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4') do
          schedules.each do |schedule|
            render_schedule_card(schedule)
          end
        end
      end

      def render_schedule_card(schedule)
        m3_card(id: "schedule_#{schedule.id}", class: 'h-full flex flex-col') do
          CardHeader do
            render_medication_icon
            m3_text(
              size: '4',
              weight: 'semibold',
              class: 'leading-tight tracking-tight text-foreground break-words'
            ) do
              schedule.medication.name
            end
          end

          CardContent(class: 'flex-grow space-y-2') do
            render_schedule_details(schedule)
          end

          CardFooter do
            render_schedule_actions(schedule)
          end
        end
      end

      def render_medication_icon
        div(class: 'w-10 h-10 rounded-xl flex items-center justify-center bg-success-light text-success mb-2') do
          render Icons::Pill.new(size: 20)
        end
      end

      def render_schedule_details(schedule)
        div(class: 'space-y-1 text-sm text-on-surface-variant') do
          render_detail_row(t('dashboard.person_schedule.dosage'), format_dosage(schedule))
          render_detail_row(t('dashboard.person_schedule.frequency'), schedule.frequency) if schedule.frequency.present?
          render_detail_row(t('dashboard.person_schedule.ends'), format_end_date(schedule)) if schedule.end_date
        end
      end

      def render_detail_row(label, value)
        p do
          strong { "#{label}: " }
          plain value.to_s
        end
      end

      def format_dosage(schedule)
        amount = schedule.dose_amount
        unit = schedule.dose_unit
        [amount, unit].compact.join(' ')
      end

      def format_end_date(schedule)
        schedule.end_date.strftime('%B %d, %Y')
      end

      def render_schedule_actions(schedule)
        div(class: 'flex h-5 items-center space-x-4 text-sm') do
          render_take_medication_link(schedule)
          if can_delete_schedule?(schedule)
            Separator(orientation: :vertical)
            render_delete_link(schedule)
          end
        end
      end

      def render_take_medication_link(schedule)
        label = if blocked_reason_for(schedule) == :out_of_stock
                  t('dashboard.person_schedule.out_of_stock')
                else
                  t('dashboard.person_schedule.on_cooldown')
                end
        render Components::Medications::TakeAction.new(
          source: schedule,
          context: { person: person, current_user: current_user },
          amount: schedule.dose_amount,
          button: {
            label: t('dashboard.person_schedule.take_now'),
            variant: :text,
            size: :sm,
            class: 'text-primary hover:underline font-medium p-0 h-auto',
            testid: "take-medication-#{schedule.id}",
            form_class: 'inline-block'
          },
          state: {
            disabled: blocked_reason_for(schedule).present?,
            label: label
          }
        )
      end

      def render_delete_link(schedule)
        AlertDialog do
          AlertDialogTrigger do
            m3_button(
              variant: :destructive_outline,
              size: :sm,
              data: { test_id: "delete-schedule-#{schedule.id}" }
            ) { t('dashboard.delete_confirmation.delete') }
          end
          AlertDialogContent do
            AlertDialogHeader do
              AlertDialogTitle { t('dashboard.delete_confirmation.delete_schedule') }
              AlertDialogDescription do
                t('dashboard.person_schedule.delete_confirmation', medication: schedule.medication.name)
              end
            end
            AlertDialogFooter do
              AlertDialogCancel { t('dashboard.delete_confirmation.cancel') }
              form_with(
                url: person_schedule_path(schedule.person, schedule),
                method: :delete,
                class: 'inline'
              ) do
                m3_button(variant: :destructive, type: :submit) { t('dashboard.delete_confirmation.delete') }
              end
            end
          end
        end
      end

      def can_delete_schedule?(schedule)
        return false unless current_user

        policy = SchedulePolicy.new(current_user, schedule)
        policy.destroy?
      end

      def blocked_reason_for(schedule)
        @blocked_reasons ||= {}
        @blocked_reasons[schedule.id] ||= MedicationStockSourceResolver
                                          .new(user: current_user, source: schedule)
                                          .blocked_reason
      end
    end
  end
end
