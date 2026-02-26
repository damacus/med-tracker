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
                 'shadow-[0_15px_40px_rgba(0,0,0,0.08)] bg-white rounded-[2rem] transition-all ' \
                 'duration-300 hover:scale-[1.02] hover:shadow-2xl group overflow-hidden'
        ) do
          render_card_header
          render_card_content
          render_card_footer
        end
      end

      private

      def status_top_border_class
        if schedule.out_of_stock?
          'border-t-rose-500'
        elsif !schedule.can_take_now?
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
            CardTitle(class: 'text-2xl font-black tracking-tight mb-1 text-slate-900') { schedule.medication.name }
            dosage_text = "#{schedule.dosage.amount.to_i}#{schedule.dosage.unit}"
            CardDescription(class: 'text-slate-600 font-bold uppercase text-[10px] tracking-widest') do
              "#{dosage_text} • #{schedule.frequency}"
            end
          end
        end
      end

      def status_badge
        return if schedule.out_of_stock?

        if schedule.can_take_now?
          Badge(variant: :success, class: 'rounded-full text-[10px] py-0.5') { t('schedules.card.ready_now') }
        else
          Badge(variant: :warning, class: 'rounded-full text-[10px] py-0.5') { t('schedules.card.waiting') }
        end
      end

      def render_card_content
        CardContent(class: 'flex-grow space-y-6 px-8') do
          div(class: 'pt-4 border-t border-slate-50 space-y-4') do
            render_date_details
            render_notes if schedule.notes.present?
            render_countdown_notice if !schedule.can_take_now? && schedule.countdown_display
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
          class: 'w-12 h-12 rounded-2xl bg-slate-50 flex items-center justify-center text-slate-400 ' \
                 'group-hover:text-primary group-hover:bg-primary/5 transition-all'
        ) do
          render Icons::Pill.new(size: 24)
        end
      end

      def render_date_details
        div(class: 'flex items-center gap-6') do
          div do
            Text(size: '1', weight: 'black', class: 'uppercase tracking-widest text-slate-300') do
              t('schedules.card.started')
            end
            Text(size: '2', weight: 'semibold', class: 'text-slate-600') do
              schedule.start_date.strftime('%b %d, %Y')
            end
          end

          if schedule.end_date
            div do
              Text(size: '1', weight: 'black', class: 'uppercase tracking-widest text-slate-300') do
                t('schedules.card.ends')
              end
              Text(size: '2', weight: 'semibold', class: 'text-slate-600') do
                schedule.end_date.strftime('%b %d, %Y')
              end
            end
          end
        end
      end

      def render_notes
        div(class: 'p-4 bg-blue-50/50 border border-blue-100 rounded-2xl') do
          div(class: 'flex items-center gap-2 mb-1') do
            render Icons::AlertCircle.new(size: 14, class: 'text-blue-600')
            Text(size: '1', weight: 'black', class: 'uppercase tracking-widest text-blue-600') do
              t('schedules.card.notes')
            end
          end
          Text(size: '2', class: 'text-blue-800 leading-relaxed') { schedule.notes }
        end
      end

      def render_countdown_notice
        div(class: 'p-4 bg-amber-50/50 border border-amber-100 rounded-2xl') do
          div(class: 'flex items-center gap-2 mb-1') do
            render Icons::AlertCircle.new(size: 14, class: 'text-amber-600')
            Text(size: '1', weight: 'black', class: 'uppercase tracking-widest text-amber-600') do
              t('schedules.card.next_dose_available')
            end
          end
          Text(size: '2', class: 'text-amber-800 font-bold') { schedule.countdown_display }
        end
      end

      def render_takes_section
        div(class: 'space-y-4 pt-2') do
          div(class: 'flex items-center justify-between') do
            Text(size: '1', weight: 'black', class: 'uppercase tracking-widest text-slate-400') do
              t('schedules.card.todays_doses')
            end
            if schedule.max_daily_doses.present?
              takes_count = todays_takes&.count ||
                            schedule.medication_takes.where(taken_at: Time.current.beginning_of_day..).count
              Badge(variant: :outline, class: 'rounded-full text-[10px]') do
                "#{takes_count}/#{schedule.max_daily_doses}"
              end
            end
          end
          render_todays_takes
        end
      end

      def render_todays_takes
        takes = todays_takes || schedule.medication_takes
                                        .where(taken_at: Time.current.beginning_of_day..)
                                        .order(taken_at: :desc)

        if takes.any?
          div(class: 'grid grid-cols-1 gap-2') do
            takes.each do |take|
              render_take_item(take)
            end
          end
        else
          Text(size: '2', weight: 'medium', class: 'italic text-slate-300 px-1') do
            t('schedules.card.no_doses_today')
          end
        end
      end

      def render_take_item(take)
        div(
          class: 'flex items-center justify-between p-3 rounded-xl bg-slate-50/50 group/item transition-colors ' \
                 'hover:bg-slate-50'
        ) do
          div(class: 'flex items-center gap-3') do
            render Icons::CheckCircle.new(size: 16, class: 'text-emerald-500')
            Text(size: '2', weight: 'bold', class: 'text-slate-700') { take.taken_at.strftime('%l:%M %p').strip }
          end
          Text(size: '1', weight: 'black', class: 'text-slate-400 uppercase tracking-tighter') do
            "#{take.amount_ml.to_i}#{schedule.dosage.unit}"
          end
        end
      end

      def render_take_medication_button
        if schedule.can_administer?
          form_with(
            url: take_medication_person_schedule_path(person, schedule),
            method: :post,
            class: 'flex-1',
            data: { controller: 'optimistic-take', action: 'submit->optimistic-take#submit' }
          ) do
            render RubyUI::Button.new(
              type: :submit,
              variant: :primary,
              size: :lg,
              class: 'w-full rounded-xl py-6 font-bold shadow-lg shadow-primary/20 hover:shadow-xl ' \
                     'hover:shadow-primary/30',
              data: { optimistic_take_target: 'button', testid: "take-schedule-#{schedule.id}" }
            ) do
              plain take_label('schedules')
            end
          end
        else
          render_disabled_button_with_reason
        end
      end

      def render_disabled_button_with_reason
        reason = schedule.administration_blocked_reason
        label = reason == :out_of_stock ? t('schedules.card.out_of_stock') : take_label('schedules')
        render Button.new(
          variant: :secondary,
          size: :lg,
          disabled: true,
          class: 'flex-1 rounded-xl py-6 opacity-50 grayscale',
          data: { testid: "take-schedule-#{schedule.id}-disabled" }
        ) { label }
      end

      def own_dose?
        return true if current_user.nil?

        current_user.person == person
      end

      def take_label(scope)
        own_dose? ? t("#{scope}.card.take") : t("#{scope}.card.give")
      end

      def render_schedule_actions
        div(class: 'flex items-center gap-2 w-full') do
          render_take_medication_button

          if view_context.current_user&.administrator?
            Link(
              href: edit_person_schedule_path(person, schedule),
              variant: :outline,
              class: 'w-12 h-12 p-0 rounded-xl border-slate-100 flex items-center justify-center ' \
                     'text-slate-400 hover:text-slate-600',
              data: { testid: "edit-schedule-#{schedule.id}" }
            ) do
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
                   class: 'w-12 h-12 p-0 rounded-xl text-slate-300 hover:text-destructive hover:bg-destructive/5',
                   data: { testid: "delete-schedule-#{schedule.id}" }) do
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
