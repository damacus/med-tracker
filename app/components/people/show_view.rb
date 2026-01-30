# frozen_string_literal: true

module Components
  module People
    # Person show view component
    class ShowView < Components::Base
      include Phlex::Rails::Helpers::TurboFrameTag
      include Phlex::Rails::Helpers::FormWith

      attr_reader :person, :prescriptions, :person_medicines, :editing

      def initialize(person:, prescriptions:, person_medicines: nil, editing: false)
        @person = person
        @prescriptions = prescriptions
        @person_medicines = person_medicines || person.person_medicines
        @editing = editing
        super()
      end

      def view_template
        div(class: 'container mx-auto px-4 py-8 pb-24 md:pb-8 max-w-7xl') do
          render_person_details
          render_prescriptions_section
          render_my_medicines_section
        end
      end

      private

      def render_person_details
        turbo_frame_tag "person_#{person.id}" do
          Card(class: 'mb-8') do
            if editing
              render_edit_form
            else
              render_person_info
            end
          end
        end
      end

      def render_edit_form
        form_with(model: person, class: 'space-y-6', data: { controller: 'auto-submit' }) do |f|
          CardHeader do
            Heading(level: 2, size: '6', class: 'font-semibold leading-none tracking-tight') { 'Edit Person' }
          end

          CardContent(class: 'space-y-4') do
            FormField do
              FormFieldLabel(for: 'person_name') { 'Name' }
              render f.text_field(
                :name,
                class: 'flex h-9 w-full rounded-md border bg-background px-3 py-1 text-sm shadow-sm ' \
                       'border-border focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring',
                data: { action: 'change->auto-submit#submit' }
              )
            end

            FormField do
              FormFieldLabel(for: 'person_date_of_birth') { 'Date of Birth' }
              render f.date_field(
                :date_of_birth,
                class: 'flex h-9 w-full rounded-md border bg-background px-3 py-1 text-sm shadow-sm ' \
                       'border-border focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring',
                data: { action: 'change->auto-submit#submit' }
              )
            end
          end

          CardFooter(class: 'flex gap-2') do
            Button(type: :submit, variant: :primary) { 'Save' }
            Link(href: person_path(person), variant: :outline) { 'Cancel' }
          end
        end
      end

      def render_person_info
        CardHeader do
          div(class: 'space-y-4') do
            # Mobile: Quick actions first for thumb accessibility
            div(class: 'md:hidden') do
              div(class: 'flex flex-wrap gap-2 mb-4') do
                render_person_quick_actions
              end
            end

            Heading(level: 1, size: '7', class: 'font-semibold tracking-tight') { person.name }
            CardDescription do
              div(class: 'space-y-1') do
                p { "Born: #{person.date_of_birth.strftime('%B %d, %Y')}" }
                p { "Age: #{person.age}" }
              end
            end

            # Desktop: Actions below info
            div(class: 'hidden md:flex flex-wrap gap-2 pt-2') do
              render_person_quick_actions
            end
          end
        end
      end

      def render_person_quick_actions
        Link(
          href: new_person_prescription_path(person),
          variant: :primary,
          class: 'min-h-[44px]',
          data: { turbo_stream: true }
        ) { 'Add Prescription' }
        if view_context.policy(PersonMedicine.new(person: person)).create?
          Link(
            href: new_person_person_medicine_path(person),
            variant: :primary,
            class: 'min-h-[44px]',
            data: { turbo_stream: true }
          ) { 'Add Medicine' }
        end
        if view_context.policy(person).update?
          Link(href: person_path(person, editing: true), variant: :outline, class: 'min-h-[44px]') { 'Edit Person' }
        end
        Link(href: people_path, variant: :outline, class: 'min-h-[44px]') { 'Back' }
      end

      def render_prescriptions_section
        div(class: 'space-y-6') do
          render_prescriptions_header
          turbo_frame_tag 'prescription_modal'
          render_prescriptions_grid
        end
      end

      def render_prescriptions_header
        Heading(level: 2, class: 'mb-6') { 'Prescriptions' }
      end

      def render_prescriptions_grid
        div(id: 'prescriptions', class: 'grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6') do
          if prescriptions.any?
            prescriptions.each do |prescription|
              render Components::Prescriptions::Card.new(prescription: prescription, person: person)
            end
          else
            render_empty_state
          end
        end
      end

      def render_empty_state
        div(class: 'col-span-full') do
          Card(class: 'text-center py-12') do
            CardContent do
              Text(size: '2', class: 'text-muted-foreground') { 'No prescriptions yet.' }
              Link(
                href: new_person_prescription_path(person),
                variant: :primary,
                data: { turbo_stream: true }
              ) { 'Add First Prescription' }
            end
          end
        end
      end

      def render_my_medicines_section
        div(class: 'space-y-6 mt-8') do
          render_my_medicines_header
          turbo_frame_tag 'person_medicine_modal'
          render_my_medicines_grid
        end
      end

      def render_my_medicines_header
        Heading(level: 2, class: 'mb-6') { 'My Medicines' }
      end

      def render_my_medicines_grid
        # Filter person_medicines based on policy
        accessible_medicines = person_medicines.select do |pm|
          view_context.policy(pm).show?
        end

        div(id: 'person_medicines', class: 'grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6') do
          if accessible_medicines.any?
            accessible_medicines.each do |person_medicine|
              render Components::PersonMedicines::Card.new(person_medicine: person_medicine, person: person)
            end
          else
            render_my_medicines_empty_state
          end
        end
      end

      def render_my_medicines_empty_state
        div(class: 'col-span-full') do
          Card(class: 'text-center py-12') do
            CardContent do
              Text(size: '2', class: 'text-muted-foreground') { 'No medicines added yet.' }
              Text(size: '2') { 'Add vitamins, supplements, or over-the-counter medicines here.' }
              if view_context.policy(PersonMedicine.new(person: person)).create?
                Link(
                  href: new_person_person_medicine_path(person),
                  variant: :primary,
                  data: { turbo_stream: true }
                ) { 'Add First Medicine' }
              end
            end
          end
        end
      end
    end
  end
end
