# frozen_string_literal: true

module Components
  module Dashboard
    # Renders a prescription row for desktop table view
    class PrescriptionRow < Components::Base
      include Phlex::Rails::Helpers::FormWith
      include PrescriptionHelpers

      attr_reader :person, :prescription, :url_helpers, :current_user

      def initialize(person:, prescription:, url_helpers: nil, current_user: nil)
        @person = person
        @prescription = prescription
        @url_helpers = url_helpers
        @current_user = current_user
        super()
      end

      def view_template
        TableRow(id: "prescription_#{prescription.id}") do
          render_person_cell
          render_medicine_cell
          TableCell { format_dosage }
          TableCell { format_quantity }
          TableCell { prescription.frequency || '—' }
          TableCell { format_end_date }
          TableCell(class: 'text-center') { render_actions }
        end
      end

      private

      def render_person_cell
        TableCell(class: 'font-medium') do
          div(class: 'flex items-center gap-2') do
            render_person_avatar
            span(class: 'font-semibold text-slate-900') { person.name }
          end
        end
      end

      def render_person_avatar
        Avatar(size: :sm) do
          AvatarFallback { '👤' }
        end
      end

      def render_medicine_cell
        TableCell do
          div(class: 'flex justify-between items-center w-full gap-2') do
            div(class: 'flex items-center gap-2') do
              render_medicine_icon
              span(class: 'font-medium') { prescription.medicine.name }
            end
            render Components::Shared::StockBadge.new(medicine: prescription.medicine)
          end
        end
      end

      def render_medicine_icon
        div(class: 'w-8 h-8 rounded-lg flex items-center justify-center bg-success-light text-success flex-shrink-0') do
          render Icons::Pill.new(size: 16)
        end
      end

      def render_actions
        div(class: 'flex items-center justify-center gap-2') do
          render_take_now_button if url_helpers
          render_delete_button if can_delete?
        end
      end

      def render_take_now_button
        if prescription.can_administer?
          form_with(
            url: url_helpers.prescription_medication_takes_path(prescription),
            method: :post,
            class: 'inline-block'
          ) do
            Button(
              type: :submit,
              variant: :success_outline,
              size: :sm,
              data: { test_id: "take-medicine-#{prescription.id}" }
            ) { 'Take Now' }
          end
        else
          render_disabled_take_button
        end
      end

      def render_disabled_take_button
        reason = prescription.administration_blocked_reason
        label = reason == :out_of_stock ? 'Out of Stock' : 'On Cooldown'
        Button(
          variant: :secondary,
          size: :sm,
          disabled: true,
          data: { test_id: "take-medicine-#{prescription.id}" }
        ) { label }
      end

      def render_delete_button
        render Components::Dashboard::DeleteConfirmationDialog.new(
          prescription: prescription,
          url_helpers: url_helpers
        )
      end
    end
  end
end
