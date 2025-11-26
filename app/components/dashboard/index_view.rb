# frozen_string_literal: true

module Components
  module Dashboard
    # Dashboard index view component that renders the main dashboard page
    class IndexView < Components::Base
      include Pundit::Authorization
      include Phlex::Rails::Helpers::ButtonTo

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
        div(class: 'flex justify-between items-center mb-8') do
          h1(class: 'text-4xl font-bold text-slate-900') { 'Dashboard' }
          render_quick_actions
        end
      end

      def render_quick_actions
        div(class: 'flex gap-3') do
          a(
            href: url_helpers&.new_medicine_path || '#',
            class: button_primary_classes
          ) { 'Add Medicine' }
          a(
            href: url_helpers&.new_user_path || '#',
            class: button_secondary_classes
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
              CardTitle(class: 'text-lg font-medium text-slate-600') { title }
              render_stat_icon(icon_type)
            end
          end
          CardContent do
            p(class: 'text-4xl font-bold text-slate-900') { value.to_s }
          end
        end
      end

      def render_stat_icon(icon_type)
        div(class: 'w-10 h-10 rounded-xl flex items-center justify-center bg-blue-100 text-blue-700') do
          case icon_type
          when 'users'
            render_users_icon
          when 'pill'
            render_pill_icon
          end
        end
      end

      def render_users_icon
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
          s.path(d: 'M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2')
          s.circle(cx: '9', cy: '7', r: '4')
          s.path(d: 'M22 21v-2a4 4 0 0 0-3-3.87')
          s.path(d: 'M16 3.13a4 4 0 0 1 0 7.75')
        end
      end

      def render_pill_icon
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
          s.path(d: 'M10.5 20.5 10 21a2 2 0 0 1-2.828 0L4.343 18.172a2 2 0 0 1 0-2.828l.5-.5')
          s.path(d: 'm7 17-5-5')
          s.path(d: 'M13.5 3.5 14 3a2 2 0 0 1 2.828 0l2.829 2.828a2 2 0 0 1 0 2.829l-.5.5')
          s.path(d: 'm17 7 5 5')
          s.circle(cx: '12', cy: '12', r: '2')
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
          h2(class: 'text-2xl font-bold text-slate-900') { 'Medication Schedule' }

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
        div(class: 'w-8 h-8 rounded-full flex items-center justify-center bg-slate-100 text-slate-700 flex-shrink-0') do
          svg(
            xmlns: 'http://www.w3.org/2000/svg',
            width: '16',
            height: '16',
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

      def render_medicine_icon_small
        div(class: 'w-8 h-8 rounded-lg flex items-center justify-center bg-green-100 text-green-700 flex-shrink-0') do
          svg(
            xmlns: 'http://www.w3.org/2000/svg',
            width: '16',
            height: '16',
            viewBox: '0 0 24 24',
            fill: 'none',
            stroke: 'currentColor',
            stroke_width: '2',
            stroke_linecap: 'round',
            stroke_linejoin: 'round'
          ) do |s|
            s.path(d: 'M10.5 20.5 10 21a2 2 0 0 1-2.828 0L4.343 18.172a2 2 0 0 1 0-2.828l.5-.5')
            s.path(d: 'm7 17-5-5')
            s.path(d: 'M13.5 3.5 14 3a2 2 0 0 1 2.828 0l2.829 2.828a2 2 0 0 1 0 2.829l-.5.5')
            s.path(d: 'm17 7 5 5')
            s.circle(cx: '12', cy: '12', r: '2')
          end
        end
      end

      def render_prescription_actions(prescription)
        div(class: 'flex items-center justify-center gap-2') do
          if url_helpers
            button_to(
              url_helpers.prescription_medication_takes_path(prescription),
              method: :post,
              class: take_now_badge_classes,
              data: { test_id: "take-medicine-#{prescription.id}" }
            ) { 'Take Now' }
          end

          render_delete_dialog(prescription) if can_delete_prescription?(prescription)
        end
      end

      def render_delete_dialog(prescription)
        AlertDialog do
          AlertDialogTrigger do
            button(
              class: delete_badge_classes,
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
              button_to(
                url_helpers.person_prescription_path(prescription.person, prescription),
                method: :delete,
                form: { data: { turbo_frame: '_top' } },
                class: 'inline-flex items-center justify-center rounded-md text-sm font-medium px-4 py-2 ' \
                       'bg-destructive text-destructive-foreground hover:bg-destructive/90',
                data: { test_id: "confirm-delete-#{prescription.id}" }
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
          h2(class: 'text-2xl font-bold text-slate-900') { 'Medication Schedule' }
          Card(class: 'text-center py-12') do
            CardContent do
              p(class: 'text-xl font-semibold text-slate-700 mb-2') { 'No active prescriptions found' }
              p(class: 'text-slate-600') { 'Add prescriptions to see them here' }
            end
          end
        end
      end
    end
  end
end
