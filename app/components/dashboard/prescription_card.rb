# frozen_string_literal: true

module Components
  module Dashboard
    # Renders a prescription card for mobile view
    class PrescriptionCard < Components::Base
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
        Card(class: 'p-4', id: "prescription_#{prescription.id}") do
          render_card_content
          render_card_actions
        end
      end

      private

      def render_card_content
        div(class: 'flex items-start justify-between gap-3') do
          div(class: 'flex-1 min-w-0') do
            render_header
            render_medicine_info
            render_details
          end
        end
      end

      def render_header
        div(class: 'flex justify-between items-start w-full mb-2') do
          div(class: 'flex items-center gap-2') do
            render_person_avatar
            span(class: 'font-semibold text-slate-900 truncate') { person.name }
          end
          render Components::Shared::StockBadge.new(medicine: prescription.medicine)
        end
      end

      def render_person_avatar
        Avatar(size: :sm) do
          AvatarFallback { '👤' }
        end
      end

      def render_medicine_info
        div(class: 'flex items-center gap-2 mb-3') do
          render_medicine_icon
          span(class: 'font-medium text-slate-700') { prescription.medicine.name }
        end
      end

      def render_medicine_icon
        div(class: 'w-8 h-8 rounded-lg flex items-center justify-center bg-success-light text-success flex-shrink-0') do
          render Icons::Pill.new(size: 16)
        end
      end

      def render_details
        div(class: 'grid grid-cols-2 gap-2 text-sm text-slate-600') do
          render_detail('Dosage', format_dosage)
          render_detail('Quantity', format_quantity)
          render_detail('Frequency', prescription.frequency || '—')
          render_detail('Ends', format_end_date)
        end
      end

      def render_detail(label, value)
        div do
          span(class: 'text-slate-500') { "#{label}: " }
          span(class: 'font-medium') { value }
        end
      end

      def render_card_actions
        div(class: 'mt-4 flex flex-wrap gap-2') do
          render_take_now_button if url_helpers
          render_delete_button if can_delete?
        end
      end

      def render_take_now_button
        form_with(
          url: url_helpers.prescription_medication_takes_path(prescription),
          method: :post,
          class: 'inline-block'
        ) do
          Button(
            type: :submit,
            variant: :success_outline,
            size: :md,
            data: { test_id: "take-medicine-#{prescription.id}" }
          ) { 'Take Now' }
        end
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
