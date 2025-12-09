# frozen_string_literal: true

module Components
  module Dashboard
    # Dashboard index view component that renders the main dashboard page
    class IndexView < Components::Base
      include Pundit::Authorization
      include Phlex::Rails::Helpers::ButtonTo
      include Phlex::Rails::Helpers::FormWith

      attr_reader :people, :active_prescriptions, :upcoming_prescriptions, :url_helpers, :current_user

      def initialize(people:, active_prescriptions:, upcoming_prescriptions:, url_helpers: nil, current_user: nil)
        @people = people
        @active_prescriptions = active_prescriptions
        @upcoming_prescriptions = upcoming_prescriptions
        @url_helpers = url_helpers
        @current_user = current_user
        super()
      end

      def view_template
        div(class: 'container mx-auto px-4 py-8', data: { testid: 'dashboard' }) do
          render_header
          render_stats_section
          render_prescriptions_table
        end
      end

      private

      def render_header
        div(class: 'flex flex-col sm:flex-row sm:justify-between sm:items-center gap-4 mb-8') do
          Heading(level: 1) { 'Dashboard' }
          render_quick_actions
        end
      end

      def render_quick_actions
        div(class: 'flex flex-col sm:flex-row gap-2 sm:gap-3 w-full sm:w-auto') do
          Link(
            href: url_helpers&.new_medicine_path || '#',
            class: "#{button_primary_classes} w-full sm:w-auto justify-center min-h-[44px]"
          ) { 'Add Medicine' }
          Link(
            href: url_helpers&.new_person_path || '#',
            class: "#{button_secondary_classes} w-full sm:w-auto justify-center min-h-[44px]"
          ) { 'Add Person' }
        end
      end

      def render_stats_section
        div(class: 'grid grid-cols-1 md:grid-cols-2 gap-6 mb-8') do
          render_stat_card('People', people.count, 'users')
          render_stat_card('Active Prescriptions', active_prescriptions.count, 'pill')
        end
      end

      def render_stat_card(title, value, icon_type)
        Card(class: 'h-full') do
          CardHeader do
            div(class: 'flex items-center justify-between') do
              Heading(level: 2, size: '4', class: 'font-medium text-slate-600') { title }
              render_stat_icon(icon_type)
            end
          end
          CardContent do
            Text(size: '8', weight: 'bold', class: 'text-slate-900') { value.to_s }
          end
        end
      end

      def render_stat_icon(icon_type)
        div(class: 'w-10 h-10 rounded-xl flex items-center justify-center bg-blue-100 text-blue-700') do
          case icon_type
          when 'users'
            render Icons::Users.new(size: 20)
          when 'pill'
            render Icons::Pill.new(size: 20)
          end
        end
      end

      def button_primary_classes
        'inline-flex items-center justify-center rounded-md font-medium transition-colors ' \
          'px-4 py-2 h-9 text-sm bg-primary text-primary-foreground shadow hover:bg-primary/90'
      end

      def button_secondary_classes
        'inline-flex items-center justify-center rounded-md font-medium transition-colors ' \
          'px-4 py-2 h-9 text-sm bg-secondary text-secondary-foreground shadow-sm hover:bg-secondary/80'
      end

      def take_now_badge_classes
        'inline-flex items-center justify-center rounded-full text-xs font-medium transition-colors ' \
          'px-3 py-1 bg-green-100 text-green-700 hover:bg-green-200'
      end

      def delete_badge_classes
        'inline-flex items-center justify-center rounded-full text-xs font-medium transition-colors ' \
          'px-3 py-1 bg-red-100 text-red-700 hover:bg-red-200'
      end

      def render_prescriptions_table
        return render_empty_state if upcoming_prescriptions.empty?

        div(class: 'space-y-4') do
          Heading(level: 2) { 'Medication Schedule' }

          # Mobile card layout
          div(class: 'md:hidden space-y-3') do
            upcoming_prescriptions.each do |person, prescriptions|
              prescriptions.each do |prescription|
                render_prescription_card(person, prescription)
              end
            end
          end

          # Desktop table layout
          div(class: 'hidden md:block') do
            Table do
              TableHeader do
                TableRow do
                  TableHead { 'Person' }
                  TableHead { 'Medicine' }
                  TableHead { 'Dosage' }
                  TableHead { 'Frequency' }
                  TableHead { 'End Date' }
                  TableHead(class: 'text-center') { 'Actions' }
                end
              end

              TableBody do
                upcoming_prescriptions.each do |person, prescriptions|
                  prescriptions.each do |prescription|
                    render_prescription_row(person, prescription)
                  end
                end
              end
            end
          end
        end
      end

      def render_prescription_card(person, prescription)
        Card(class: 'p-4', id: "prescription_#{prescription.id}") do
          div(class: 'flex items-start justify-between gap-3') do
            div(class: 'flex-1 min-w-0') do
              # Person and medicine info
              div(class: 'flex items-center gap-2 mb-2') do
                render_person_avatar_small
                span(class: 'font-semibold text-slate-900 truncate') { person.name }
              end

              div(class: 'flex items-center gap-2 mb-3') do
                render_medicine_icon_small
                span(class: 'font-medium text-slate-700') { prescription.medicine.name }
              end

              # Details in a compact grid
              div(class: 'grid grid-cols-2 gap-2 text-sm text-slate-600') do
                div do
                  span(class: 'text-slate-500') { 'Dosage: ' }
                  span(class: 'font-medium') { format_dosage(prescription) }
                end
                div do
                  span(class: 'text-slate-500') { 'Frequency: ' }
                  span(class: 'font-medium') { prescription.frequency || '—' }
                end
                div(class: 'col-span-2') do
                  span(class: 'text-slate-500') { 'Ends: ' }
                  span(class: 'font-medium') { format_end_date(prescription) }
                end
              end
            end
          end

          # Actions at bottom with full-width Take Now button
          div(class: 'mt-4 flex gap-2') do
            if url_helpers
              form_with(
                url: url_helpers.prescription_medication_takes_path(prescription),
                method: :post,
                class: 'flex-1'
              ) do
                Button(
                  type: :submit,
                  variant: :primary,
                  class: 'w-full min-h-[44px]',
                  data: { test_id: "take-medicine-#{prescription.id}" }
                ) { 'Take Now' }
              end
            end

            render_delete_button_mobile(prescription) if can_delete_prescription?(prescription)
          end
        end
      end

      def render_delete_button_mobile(prescription)
        render_delete_confirmation_dialog(prescription, button_class: delete_badge_classes)
      end

      def render_prescription_row(person, prescription)
        TableRow(id: "prescription_#{prescription.id}") do
          TableCell(class: 'font-medium') do
            div(class: 'flex items-center gap-2') do
              render_person_avatar_small
              span(class: 'font-semibold text-slate-900') { person.name }
            end
          end

          TableCell do
            div(class: 'flex items-center gap-2') do
              render_medicine_icon_small
              span(class: 'font-medium') { prescription.medicine.name }
            end
          end

          TableCell { format_dosage(prescription) }
          TableCell { prescription.frequency || '—' }
          TableCell { format_end_date(prescription) }

          TableCell(class: 'text-center') do
            render_prescription_actions(prescription)
          end
        end
      end

      def render_person_avatar_small
        Avatar(size: :sm) do
          AvatarFallback { '👤' }
        end
      end

      def render_medicine_icon_small
        div(class: 'w-8 h-8 rounded-lg flex items-center justify-center bg-green-100 text-green-700 flex-shrink-0') do
          render Icons::Pill.new(size: 16)
        end
      end

      def render_prescription_actions(prescription)
        div(class: 'flex items-center justify-center gap-2') do
          if url_helpers
            form_with(
              url: url_helpers.prescription_medication_takes_path(prescription),
              method: :post,
              class: 'inline-block'
            ) do
              Button(
                type: :submit,
                variant: :primary,
                size: :sm,
                class: take_now_badge_classes,
                data: { test_id: "take-medicine-#{prescription.id}" }
              ) { 'Take Now' }
            end
          end

          render_delete_dialog(prescription) if can_delete_prescription?(prescription)
        end
      end

      def render_delete_dialog(prescription)
        render_delete_confirmation_dialog(prescription, button_class: delete_badge_classes)
      end

      def render_delete_confirmation_dialog(prescription, button_class:)
        AlertDialog do
          AlertDialogTrigger do
            Button(
              variant: :destructive,
              size: :sm,
              class: button_class,
              data: { test_id: "delete-prescription-#{prescription.id}" }
            ) { 'Delete' }
          end

          AlertDialogContent do
            AlertDialogHeader do
              AlertDialogTitle { 'Delete Prescription?' }
              AlertDialogDescription do
                plain "Are you sure you want to delete #{prescription.medicine.name} "
                plain "for #{prescription.person.name}? This action cannot be undone."
              end
            end

            AlertDialogFooter do
              AlertDialogCancel { 'Cancel' }
              Link(
                href: url_helpers.person_prescription_path(prescription.person, prescription),
                variant: :destructive,
                data: {
                  turbo_method: :delete,
                  turbo_frame: '_top',
                  test_id: "confirm-delete-#{prescription.id}",
                  action: 'click->ruby-ui--alert-dialog#close'
                }
              ) { 'Delete' }
            end
          end
        end
      end

      def format_dosage(prescription)
        amount = prescription.dosage&.amount
        unit = prescription.dosage&.unit
        return '—' unless amount && unit

        # Format amount as integer if it's a whole number, otherwise show minimal decimals
        formatted_amount = amount == amount.to_i ? amount.to_i : amount
        "#{formatted_amount}#{unit}"
      end

      def format_end_date(prescription)
        prescription.end_date ? prescription.end_date.strftime('%b %d, %Y') : '—'
      end

      def can_delete_prescription?(prescription)
        return false unless current_user

        policy = PrescriptionPolicy.new(current_user, prescription)
        policy.destroy?
      end

      def render_empty_state
        div(class: 'space-y-6') do
          Heading(level: 2) { 'Medication Schedule' }
          Card(class: 'text-center py-12') do
            CardContent do
              Text(size: '5', weight: 'semibold', class: 'text-slate-700 mb-2') { 'No active prescriptions found' }
              Text(class: 'text-slate-600') { 'Add prescriptions to see them here' }
            end
          end
        end
      end
    end
  end
end
