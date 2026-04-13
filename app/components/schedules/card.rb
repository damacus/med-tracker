# frozen_string_literal: true

module Components
  module Schedules
    # Renders a schedule card with medication details and take medication form
    class Card < Components::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::ButtonTo

      attr_reader :schedule, :person, :todays_takes, :current_user

      def initialize(schedule:, person:, todays_takes: nil, current_user: nil)
        @schedule = schedule
        @person = person
        @todays_takes = todays_takes
        @current_user = current_user
        super()
      end

      def view_template
        render RubyUI::Card.new(
          id: "schedule_#{schedule.id}",
          class: "h-full flex flex-col border-none border-t-4 #{status_top_border_class} " \
                 'shadow-[0_15px_40px_rgba(0,0,0,0.08)] bg-surface-container-lowest rounded-[2rem] transition-all ' \
                 'duration-300 hover:scale-[1.02] hover:shadow-2xl group overflow-hidden'
        ) do
          render_card_header
          render_card_content
          render_card_footer
        end
      end

      private

      def status_top_border_class
        if out_of_stock?
          'border-t-rose-500'
        elsif !can_take_now?
          'border-t-amber-500'
        else
          'border-t-primary'
        end
      end

      def render_card_header
        CardHeader(class: 'pb-4 pt-8 px-8') do
          div(class: 'flex justify-between items-start mb-4') do
            render_medication_icon
            div(class: 'flex flex-col items-end gap-2') do
              render Components::Shared::StockBadge.new(medication: schedule.medication)
              status_badge
            end
          end
          div do
            CardTitle(class: 'text-2xl font-black tracking-tight mb-1 text-foreground') { schedule.medication.name }
            dosage_text = "#{schedule.dose_amount.to_i}#{schedule.dose_unit}"
            CardDescription(class: 'text-muted-foreground font-bold uppercase text-[10px] tracking-widest') do
              "#{dosage_text} • #{schedule.frequency}"
            end
          end
        end
      end

      def status_badge
        return if out_of_stock?

        if can_take_now?
          Badge(variant: :success, class: 'rounded-full text-[10px] py-0.5') { t('schedules.card.ready_now') }
        else
          Badge(variant: :warning, class: 'rounded-full text-[10px] py-0.5') { t('schedules.card.waiting') }
        end
      end

      def render_card_content
        CardContent(class: 'flex-grow space-y-6 px-8') do
          div(class: 'pt-4 border-t border-border space-y-4') do
            render_date_details
            render_notes if schedule.notes.present?
            render_countdown_notice if !can_take_now? && schedule.countdown_display
            render_takes_section
          end
        end
      end

      def render_card_footer
        CardFooter(class: 'px-8 pb-8 pt-2') do
          render_schedule_actions
        end
      end

      def render_medication_icon
        div(
          class: 'w-12 h-12 rounded-2xl bg-surface-container-low flex items-center ' \
                 'justify-center text-muted-foreground ' \
                 'group-hover:text-primary group-hover:bg-primary/5 transition-all'
        ) do
          render Icons::Pill.new(size: 24)
        end
      end

      def render_date_details
        div(class: 'flex items-center gap-6') do
          div do
            Text(size: '1', weight: 'black', class: 'uppercase tracking-widest text-muted-foreground') do
              t('schedules.card.started')
            end
            Text(size: '2', weight: 'semibold', class: 'text-muted-foreground') do
              schedule.start_date.strftime('%b %d, %Y')
            end
          end

          if schedule.end_date
            div do
              Text(size: '1', weight: 'black', class: 'uppercase tracking-widest text-muted-foreground') do
                t('schedules.card.ends')
              end
              Text(size: '2', weight: 'semibold', class: 'text-muted-foreground') do
                schedule.end_date.strftime('%b %d, %Y')
              end
            end
          end
        end
      end

      def render_notes
        div(class: 'p-4 bg-primary-container border border-primary/20 rounded-2xl') do
          div(class: 'flex items-center gap-2 mb-1') do
            render Icons::AlertCircle.new(size: 14, class: 'text-on-primary-container')
            Text(size: '1', weight: 'black', class: 'uppercase tracking-widest text-on-primary-container') do
              t('schedules.card.notes')
            end
          end
          Text(size: '2', class: 'text-on-primary-container leading-relaxed') { schedule.notes }
        end
      end

      def render_countdown_notice
        div(class: 'p-4 bg-warning-container border border-warning/20 rounded-2xl') do
          div(class: 'flex items-center gap-2 mb-1') do
            render Icons::AlertCircle.new(size: 14, class: 'text-on-warning-container')
            Text(size: '1', weight: 'black', class: 'uppercase tracking-widest text-on-warning-container') do
              t('schedules.card.next_dose_available')
            end
          end
          Text(size: '2', class: 'text-on-warning-container font-bold') { schedule.countdown_display }
        end
      end

      def render_takes_section
        div(class: 'space-y-4 pt-2') do
          div(class: 'flex items-center justify-between') do
            Text(size: '1', weight: 'black', class: 'uppercase tracking-widest text-muted-foreground') do
              t('schedules.card.todays_doses')
            end
            if schedule.max_daily_doses.present?
              Badge(variant: :outline, class: 'rounded-full text-[10px]') do
                "#{todays_takes_count}/#{schedule.max_daily_doses}"
              end
            end
          end
          render_todays_takes
        end
      end

      def render_todays_takes
        takes = resolved_todays_takes

        if takes.any?
          div(class: 'grid grid-cols-1 gap-2') do
            takes.each do |take|
              render_take_item(take)
            end
          end
        else
          Text(size: '2', weight: 'medium', class: 'italic text-muted-foreground px-1') do
            t('schedules.card.no_doses_today')
          end
        end
      end

      def fetch_todays_takes
        if schedule.medication_takes.loaded?
          schedule.medication_takes
                  .select { |t| t.taken_at >= Time.current.beginning_of_day }
                  .sort_by(&:taken_at)
                  .reverse
        else
          schedule.medication_takes
                  .where(taken_at: Time.current.beginning_of_day..)
                  .order(taken_at: :desc)
        end
      end

      # Cache these per-render so the card doesn't rescan or requery the same data
      # for the status badge, disabled button, dose count badge, and take history.
      def out_of_stock?
        blocked_reason == :out_of_stock
      end

      def can_take_now?
        return @can_take_now if instance_variable_defined?(:@can_take_now)

        @can_take_now = schedule.can_take_now?
      end

      def can_administer?
        blocked_reason.nil?
      end

      def blocked_reason
        return @blocked_reason if instance_variable_defined?(:@blocked_reason)

        @blocked_reason = stock_source_resolver.blocked_reason
      end

      def resolved_todays_takes
        return @resolved_todays_takes if instance_variable_defined?(:@resolved_todays_takes)

        @resolved_todays_takes = (todays_takes || fetch_todays_takes).to_a
      end

      def todays_takes_count
        @todays_takes_count ||= resolved_todays_takes.size
      end

      def render_take_item(take)
        div(
          class: 'flex items-center justify-between p-3 rounded-xl ' \
                 'bg-surface-container-low group/item transition-colors ' \
                 'hover:bg-accent'
        ) do
          div(class: 'flex items-center gap-3') do
            render Icons::CheckCircle.new(size: 16, class: 'text-on-success-container')
            div(class: 'space-y-1') do
              Text(size: '2', weight: 'bold', class: 'text-foreground') do
                take.taken_at.strftime('%l:%M %p').strip
              end
              if take.inventory_location.present?
                Text(size: '1', class: 'text-muted-foreground') { take.inventory_location.name }
              end
            end
          end
          Text(size: '1', weight: 'black', class: 'text-muted-foreground uppercase tracking-tighter') do
            "#{take.amount_ml.to_i}#{schedule.dose_unit}"
          end
        end
      end

      def render_take_medication_button
        label = if invalid_dose_configured?
                  t('schedules.card.invalid_dose')
                else
                  blocked_reason == :out_of_stock ? t('schedules.card.out_of_stock') : take_label('schedules')
                end
        render Components::Medications::TakeAction.new(
          source: schedule,
          context: { person: person, current_user: current_user },
          amount: schedule.dose_amount,
          button: {
            label: take_label('schedules'),
            variant: :primary,
            size: :lg,
            class: 'w-full rounded-xl py-6 font-bold shadow-lg shadow-primary/20 hover:shadow-xl ' \
                   'hover:shadow-primary/30',
            testid: "take-schedule-#{schedule.id}",
            form_class: 'flex-1'
          },
          state: {
            disabled: invalid_dose_configured? || !can_administer?,
            label: label
          }
        )
      end

      def own_dose?
        return true if current_user.nil?

        current_user.person == person
      end

      def take_label(scope)
        own_dose? ? t("#{scope}.card.take") : t("#{scope}.card.give")
      end

      def invalid_dose_configured?
        schedule.dose_amount.to_f <= 0
      end

      def stock_source_resolver
        @stock_source_resolver ||= MedicationStockSourceResolver.new(user: current_user, source: schedule)
      end

      def render_schedule_actions
        div(class: 'flex items-center gap-2 w-full') do
          render_take_medication_button

          if view_context.current_user&.administrator?
            Link(
              href: edit_person_schedule_path(person, schedule),
              variant: :outline,
              class: 'w-12 h-12 p-0 rounded-xl border-border flex items-center justify-center ' \
                     'text-muted-foreground hover:text-foreground',
              data: { turbo_frame: 'modal', testid: "edit-schedule-#{schedule.id}" }
            ) do
              span(class: 'sr-only') { t('schedules.card.edit', default: 'Edit schedule') }
              render Icons::Pencil.new(size: 20)
            end
            render_delete_dialog
          end
        end
      end

      def render_delete_dialog
        AlertDialog do
          AlertDialogTrigger do
            Button(variant: :ghost,
                   class: 'w-12 h-12 p-0 rounded-xl text-muted-foreground ' \
                          'hover:text-destructive hover:bg-destructive/5',
                   data: { testid: "delete-schedule-#{schedule.id}" }) do
              span(class: 'sr-only') { t('schedules.card.delete', default: 'Delete schedule') }
              render Icons::Trash.new(size: 20)
            end
          end
          AlertDialogContent(class: 'rounded-[2rem] border-none shadow-2xl') do
            AlertDialogHeader do
              AlertDialogTitle { t('schedules.card.delete_dialog.title') }
              AlertDialogDescription do
                plain t('schedules.card.delete_dialog.confirm', medication: schedule.medication.name)
              end
            end
            AlertDialogFooter do
              AlertDialogCancel(class: 'rounded-xl') { t('schedules.card.delete_dialog.cancel') }
              form_with(
                url: person_schedule_path(person, schedule),
                method: :delete,
                class: 'inline'
              ) do
                Button(variant: :destructive, type: :submit, class: 'rounded-xl shadow-lg shadow-destructive/20') do
                  t('schedules.card.delete_dialog.submit')
                end
              end
            end
          end
        end
      end
    end
  end
end
