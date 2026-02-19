# frozen_string_literal: true

module Components
  module Prescriptions
    # Renders a prescription card with medication details and take medicine form
    class Card < Components::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::ButtonTo

      attr_reader :prescription, :person, :todays_takes, :current_user

      def initialize(prescription:, person:, todays_takes: nil, current_user: nil)
        @prescription = prescription
        @person = person
        @todays_takes = todays_takes
        @current_user = current_user
        super()
      end

      def view_template
        render RubyUI::Card.new(
          id: "prescription_#{prescription.id}",
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
        if prescription.out_of_stock?
          'border-t-rose-500'
        elsif !prescription.can_take_now?
          'border-t-amber-500'
        else
          'border-t-primary'
        end
      end

      def render_card_header
        CardHeader(class: 'pb-4 pt-8 px-8') do
          div(class: 'flex justify-between items-start mb-4') do
            render_medicine_icon
            div(class: 'flex flex-col items-end gap-2') do
              render Components::Shared::StockBadge.new(medicine: prescription.medicine)
              status_badge
            end
          end
          div do
            CardTitle(class: 'text-2xl font-black tracking-tight mb-1 text-slate-900') { prescription.medicine.name }
            dosage_text = "#{prescription.dosage.amount.to_i}#{prescription.dosage.unit}"
            CardDescription(class: 'text-slate-600 font-bold uppercase text-[10px] tracking-widest') do
              "#{dosage_text} • #{prescription.frequency}"
            end
          end
        end
      end

      def status_badge
        return if prescription.out_of_stock?

        if prescription.can_take_now?
          Badge(variant: :success, class: 'rounded-full text-[10px] py-0.5') { 'Ready Now' }
        else
          Badge(variant: :warning, class: 'rounded-full text-[10px] py-0.5') { 'Waiting' }
        end
      end

      def render_card_content
        CardContent(class: 'flex-grow space-y-6 px-8') do
          div(class: 'pt-4 border-t border-slate-50 space-y-4') do
            render_date_details
            render_notes if prescription.notes.present?
            render_countdown_notice if !prescription.can_take_now? && prescription.countdown_display
            render_takes_section
          end
        end
      end

      def render_card_footer
        CardFooter(class: 'px-8 pb-8 pt-2') do
          render_prescription_actions
        end
      end

      def render_medicine_icon
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
              t('prescriptions.card.started')
            end
            Text(size: '2', weight: 'semibold', class: 'text-slate-600') do
              prescription.start_date.strftime('%b %d, %Y')
            end
          end

          if prescription.end_date
            div do
              Text(size: '1', weight: 'black', class: 'uppercase tracking-widest text-slate-300') do
                t('prescriptions.card.ends')
              end
              Text(size: '2', weight: 'semibold', class: 'text-slate-600') do
                prescription.end_date.strftime('%b %d, %Y')
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
              t('prescriptions.card.notes')
            end
          end
          Text(size: '2', class: 'text-blue-800 leading-relaxed') { prescription.notes }
        end
      end

      def render_countdown_notice
        div(class: 'p-4 bg-amber-50/50 border border-amber-100 rounded-2xl') do
          div(class: 'flex items-center gap-2 mb-1') do
            render Icons::AlertCircle.new(size: 14, class: 'text-amber-600')
            Text(size: '1', weight: 'black', class: 'uppercase tracking-widest text-amber-600') do
              t('prescriptions.card.next_dose_available')
            end
          end
          Text(size: '2', class: 'text-amber-800 font-bold') { prescription.countdown_display }
        end
      end

      def render_takes_section
        div(class: 'space-y-4 pt-2') do
          div(class: 'flex items-center justify-between') do
            Text(size: '1', weight: 'black', class: 'uppercase tracking-widest text-slate-400') do
              t('prescriptions.card.todays_doses')
            end
            if prescription.max_daily_doses.present?
              takes_count = todays_takes&.count ||
                            prescription.medication_takes.where(taken_at: Time.current.beginning_of_day..).count
              Badge(variant: :outline, class: 'rounded-full text-[10px]') do
                "#{takes_count}/#{prescription.max_daily_doses}"
              end
            end
          end
          render_todays_takes
        end
      end

      def render_todays_takes
        takes = todays_takes || prescription.medication_takes
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
            t('prescriptions.card.no_doses_today')
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
            "#{take.amount_ml.to_i}#{prescription.dosage.unit}"
          end
        end
      end

      def render_take_medicine_button
        if prescription.can_administer?
          form_with(
            url: take_medicine_person_prescription_path(person, prescription),
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
              data: { optimistic_take_target: 'button', testid: "take-prescription-#{prescription.id}" }
            ) do
              plain take_label('prescriptions')
            end
          end
        else
          render_disabled_button_with_reason
        end
      end

      def render_disabled_button_with_reason
        reason = prescription.administration_blocked_reason
        label = reason == :out_of_stock ? t('prescriptions.card.out_of_stock') : take_label('prescriptions')
        render Button.new(
          variant: :secondary,
          size: :lg,
          disabled: true,
          class: 'flex-1 rounded-xl py-6 opacity-50 grayscale',
          data: { testid: "take-prescription-#{prescription.id}-disabled" }
        ) { label }
      end

      def own_dose?
        return true if current_user.nil?

        current_user.person == person
      end

      def take_label(scope)
        own_dose? ? t("#{scope}.card.take") : t("#{scope}.card.give")
      end

      def render_prescription_actions
        div(class: 'flex items-center gap-2 w-full') do
          render_take_medicine_button

          if view_context.current_user&.administrator?
            Link(
              href: edit_person_prescription_path(person, prescription),
              variant: :outline,
              class: 'w-12 h-12 p-0 rounded-xl border-slate-100 flex items-center justify-center ' \
                     'text-slate-400 hover:text-slate-600',
              data: { testid: "edit-prescription-#{prescription.id}" }
            ) do
              svg(
                xmlns: 'http://www.w3.org/2000/svg',
                class: 'w-5 h-5',
                fill: 'none',
                viewBox: '0 0 24 24',
                stroke: 'currentColor'
              ) do |s|
                s.path(
                  stroke_linecap: 'round',
                  stroke_linejoin: 'round',
                  stroke_width: '2',
                  d: 'M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z'
                )
              end
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
                   data: { testid: "delete-prescription-#{prescription.id}" }) do
              render Icons::Trash.new(size: 20)
            end
          end
          AlertDialogContent(class: 'rounded-[2rem] border-none shadow-2xl') do
            AlertDialogHeader do
              AlertDialogTitle { t('prescriptions.card.delete_dialog.title') }
              AlertDialogDescription do
                plain t('prescriptions.card.delete_dialog.confirm', medicine: prescription.medicine.name)
              end
            end
            AlertDialogFooter do
              AlertDialogCancel(class: 'rounded-xl') { t('prescriptions.card.delete_dialog.cancel') }
              form_with(
                url: person_prescription_path(person, prescription),
                method: :delete,
                class: 'inline'
              ) do
                Button(variant: :destructive, type: :submit, class: 'rounded-xl shadow-lg shadow-destructive/20') do
                  t('prescriptions.card.delete_dialog.submit')
                end
              end
            end
          end
        end
      end
    end
  end
end
