# frozen_string_literal: true

module Components
  module Dashboard
    # Renders a person's medication schedule within the dashboard
    class PersonSchedule < Components::Base
      include Phlex::Rails::Helpers::ButtonTo
      include Pundit::Authorization

      attr_reader :person, :prescriptions, :take_medicine_url_generator, :current_user

      def initialize(person:, prescriptions:, take_medicine_url_generator: nil, current_user: nil)
        @person = person
        @prescriptions = prescriptions
        @take_medicine_url_generator = take_medicine_url_generator
        @current_user = current_user
        super()
      end

      def view_template
        div(class: 'space-y-4') do
          render_person_header
          render_prescriptions_grid
        end
      end

      private

      def render_person_header
        div(class: 'flex items-center gap-3 mb-2') do
          render_person_avatar
          div do
            Heading(level: 3) { person.name }
            Text(size: '2', weight: 'muted') { "Age: #{person.age}" }
          end
        end
      end

      def render_person_avatar
        div(class: 'w-12 h-12 rounded-full flex items-center justify-center bg-slate-100 text-slate-700') do
          render Icons::User.new(size: 24)
        end
      end

      def render_prescriptions_grid
        div(class: 'grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4') do
          prescriptions.each do |prescription|
            render_prescription_card(prescription)
          end
        end
      end

      def render_prescription_card(prescription)
        Card(id: "prescription_#{prescription.id}", class: 'h-full flex flex-col') do
          CardHeader do
            render_medicine_icon
            Text(size: '4', weight: 'semibold', class: 'leading-none tracking-tight text-slate-900') do
              prescription.medicine.name
            end
          end

          CardContent(class: 'flex-grow space-y-2') do
            render_prescription_details(prescription)
          end

          CardFooter do
            render_prescription_actions(prescription)
          end
        end
      end

      def render_medicine_icon
        div(class: 'w-10 h-10 rounded-xl flex items-center justify-center bg-success-light text-success mb-2') do
          render Icons::Pill.new(size: 20)
        end
      end

      def render_prescription_details(prescription)
        div(class: 'space-y-1 text-sm text-muted-foreground') do
          render_detail_row('Dosage', format_dosage(prescription))
          render_detail_row('Frequency', prescription.frequency) if prescription.frequency.present?
          render_detail_row('Ends', format_end_date(prescription)) if prescription.end_date
        end
      end

      def render_detail_row(label, value)
        p do
          strong { "#{label}: " }
          plain value.to_s
        end
      end

      def format_dosage(prescription)
        amount = prescription.dosage&.amount
        unit = prescription.dosage&.unit
        [amount, unit].compact.join(' ')
      end

      def format_end_date(prescription)
        prescription.end_date.strftime('%B %d, %Y')
      end

      def render_prescription_actions(prescription)
        div(class: 'flex h-5 items-center space-x-4 text-sm') do
          render_take_medicine_link(prescription)
          if can_delete_prescription?(prescription)
            Separator(orientation: :vertical)
            render_delete_link(prescription)
          end
        end
      end

      def render_take_medicine_link(prescription)
        if take_medicine_url_generator
          url = take_medicine_url_generator.call(prescription)
          Link(
            href: url,
            variant: :link,
            class: 'text-primary hover:underline font-medium',
            data: { turbo_method: :post, test_id: "take-medicine-#{prescription.id}" }
          ) { 'Take Now' }
        else
          span(class: 'text-primary font-medium', data: { test_id: "take-medicine-#{prescription.id}" }) do
            'Take Now'
          end
        end
      end

      def render_delete_link(prescription)
        AlertDialog do
          AlertDialogTrigger do
            Button(
              variant: :destructive_outline,
              size: :sm,
              data: { test_id: "delete-prescription-#{prescription.id}" }
            ) { 'Delete' }
          end
          AlertDialogContent do
            AlertDialogHeader do
              AlertDialogTitle { 'Delete Prescription' }
              AlertDialogDescription do
                "Are you sure you want to delete this prescription for #{prescription.medicine.name}? " \
                  'This action cannot be undone.'
              end
            end
            AlertDialogFooter do
              AlertDialogCancel { 'Cancel' }
              form_with(
                url: person_prescription_path(prescription.person, prescription),
                method: :delete,
                class: 'inline'
              ) do
                Button(variant: :destructive, type: :submit) { 'Delete' }
              end
            end
          end
        end
      end

      def can_delete_prescription?(prescription)
        return false unless current_user

        policy = PrescriptionPolicy.new(current_user, prescription)
        policy.destroy?
      end
    end
  end
end
