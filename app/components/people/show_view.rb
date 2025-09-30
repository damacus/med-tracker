# frozen_string_literal: true

module Components
  module People
    # Person show view component
    class ShowView < Components::Base
      include Phlex::Rails::Helpers::TurboFrameTag
      include Phlex::Rails::Helpers::DomId
      include Phlex::Rails::Helpers::FormWith

      attr_reader :person, :prescriptions, :editing

      def initialize(person:, prescriptions:, editing: false)
        @person = person
        @prescriptions = prescriptions
        @editing = editing
        super()
      end

      def view_template
        div(class: 'container mx-auto px-4 py-8 max-w-7xl') do
          render_person_details
          render_prescriptions_section
        end
      end

      private

      def render_person_details
        turbo_frame_tag dom_id(person) do
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
            CardTitle { 'Edit Person' }
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
          div(class: 'flex justify-between items-start') do
            div do
              CardTitle(class: 'text-3xl') { person.name }
              CardDescription(class: 'mt-2') do
                div(class: 'space-y-1') do
                  p { "Born: #{person.date_of_birth.strftime('%B %d, %Y')}" }
                  p { "Age: #{person.age}" }
                end
              end
            end
            div(class: 'flex gap-2') do
              Link(href: person_path(person, editing: true), variant: :outline) { 'Edit' }
              Link(href: people_path, variant: :outline) { 'Back' }
            end
          end
        end
      end

      def render_prescriptions_section
        div(class: 'space-y-6') do
          render_prescriptions_header
          turbo_frame_tag 'new_prescription'
          render_prescriptions_grid
        end
      end

      def render_prescriptions_header
        div(class: 'flex justify-between items-center') do
          h2(class: 'text-2xl font-bold text-slate-900') { 'Prescriptions' }
          Link(
            href: new_person_prescription_path(person),
            variant: :primary,
            data: { turbo_stream: true }
          ) { 'Add Prescription' }
        end
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
              p(class: 'text-slate-500 italic mb-4') { 'No prescriptions yet.' }
              Link(
                href: new_person_prescription_path(person),
                variant: :primary,
                data: { turbo_stream: true }
              ) { 'Add First Prescription' }
            end
          end
        end
      end
    end
  end
end
