# frozen_string_literal: true

module Components
  module Prescriptions
    # Renders a prescription card with medication details and take medicine form
    class Card < Components::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::ButtonTo

      attr_reader :prescription, :person, :todays_takes

      def initialize(prescription:, person:, todays_takes: nil)
        @prescription = prescription
        @person = person
        @todays_takes = todays_takes
        super()
      end

      def view_template
        RubyUI::Card(id: "prescription_#{prescription.id}", class: 'h-full flex flex-col') do
          render_card_header
          render_card_content
          render_card_footer
        end
      end

      private

      def render_card_header
        CardHeader do
          div(class: 'flex justify-between items-start w-full') do
            div do
              render_medicine_icon
              CardTitle(class: 'text-xl') { prescription.medicine.name }
              dosage_text = "#{prescription.dosage.amount.to_i} #{prescription.dosage.unit}"
              CardDescription { "#{dosage_text} • #{prescription.frequency}" }
            end
            render Components::Shared::StockBadge.new(medicine: prescription.medicine)
          end
        end
      end

      def render_card_content
        CardContent(class: 'flex-grow space-y-4') do
          render_date_details
          render_notes if prescription.notes.present?
          render_countdown_notice if !prescription.can_take_now? && prescription.countdown_display
          render_takes_section
        end
      end

      def render_card_footer
        CardFooter(class: 'flex gap-2') do
          render_prescription_actions
        end
      end

      def render_medicine_icon
        div(class: 'w-10 h-10 rounded-xl flex items-center justify-center bg-violet-100 text-violet-700 mb-2') do
          render Icons::Pill.new(size: 20)
        end
      end

      def render_date_details
        div(class: 'space-y-1 text-sm') do
          div(class: 'flex items-center gap-2') do
            Text(as: 'span', class: 'text-slate-500') { t('prescriptions.card.started') }
            Text(as: 'span', weight: 'medium', class: 'text-slate-700') do
              prescription.start_date.strftime('%B %d, %Y')
            end
          end

          if prescription.end_date
            div(class: 'flex items-center gap-2') do
              Text(as: 'span', class: 'text-slate-500') { t('prescriptions.card.ends') }
              Text(as: 'span', weight: 'medium', class: 'text-slate-700') do
                prescription.end_date.strftime('%B %d, %Y')
              end
            end
          end
        end
      end

      def render_notes
        div(class: 'p-3 bg-blue-50 border border-blue-200 rounded-md') do
          Text(size: '2', class: 'text-blue-800') do
            span(class: 'font-semibold') { t('prescriptions.card.notes') }
            plain prescription.notes
          end
        end
      end

      def render_countdown_notice
        div(class: 'p-3 bg-amber-50 border border-amber-200 rounded-md') do
          Text(size: '2', class: 'text-amber-800') do
            span(class: 'font-semibold') { t('prescriptions.card.next_dose_available') }
            plain prescription.countdown_display
          end
        end
      end

      def render_takes_section
        div(class: 'space-y-3') do
          Heading(level: 4, size: '2', class: 'font-semibold text-slate-700') { t('prescriptions.card.todays_doses') }
          render_todays_takes
        end
      end

      def render_todays_takes
        takes = todays_takes || prescription.medication_takes
                                            .where(taken_at: Time.current.beginning_of_day..)
                                            .order(taken_at: :desc)

        if takes.any?
          div(class: 'space-y-2') do
            takes.each do |take|
              render_take_item(take)
            end
          end
        else
          Text(size: '2', weight: 'muted', class: 'italic') { t('prescriptions.card.no_doses_today') }
        end
      end

      def render_take_item(take)
        div(class: 'flex items-center gap-2 text-sm') do
          render Icons::CheckCircle.new(size: 16, class: 'w-4 h-4 text-success')
          Text(as: 'span', weight: 'medium', class: 'text-slate-700') { take.taken_at.strftime('%l:%M %p').strip }
          Text(as: 'span', class: 'text-slate-500') { "#{take.amount_ml.to_i} #{prescription.dosage.unit}" }
        end
      end

      def render_take_medicine_button
        if prescription.can_administer?
          form_with(
            url: take_medicine_person_prescription_path(person, prescription),
            method: :post,
            class: 'inline-block',
            data: { controller: 'optimistic-take', action: 'submit->optimistic-take#submit' }
          ) do
            Button(
              type: :submit,
              variant: :primary,
              size: :md,
              class: 'inline-flex items-center gap-1 min-w-[80px]',
              data: { optimistic_take_target: 'button' }
            ) do
              plain t('prescriptions.card.take')
            end
          end
        else
          render_disabled_button_with_reason
        end
      end

      def render_disabled_button_with_reason
        reason = prescription.administration_blocked_reason
        label = reason == :out_of_stock ? t('prescriptions.card.out_of_stock') : t('prescriptions.card.take')
        Button(variant: :secondary, size: :md, disabled: true) { label }
      end

      def render_prescription_actions
        render_take_medicine_button
        return unless view_context.current_user&.administrator?

        Link(href: edit_person_prescription_path(person, prescription), variant: :outline) do
          t('prescriptions.card.edit')
        end
        render_delete_dialog
      end

      def render_delete_dialog
        AlertDialog do
          AlertDialogTrigger do
            Button(variant: :destructive_outline, size: :md) { t('prescriptions.card.delete') }
          end
          AlertDialogContent do
            AlertDialogHeader do
              AlertDialogTitle { t('prescriptions.card.delete_dialog.title') }
              AlertDialogDescription do
                plain t('prescriptions.card.delete_dialog.confirm', medicine: prescription.medicine.name)
              end
            end
            AlertDialogFooter do
              AlertDialogCancel { t('prescriptions.card.delete_dialog.cancel') }
              form_with(
                url: person_prescription_path(person, prescription),
                method: :delete,
                class: 'inline'
              ) do
                Button(variant: :destructive, type: :submit) { t('prescriptions.card.delete_dialog.submit') }
              end
            end
          end
        end
      end
    end
  end
end
