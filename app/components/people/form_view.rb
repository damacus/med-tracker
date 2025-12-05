# frozen_string_literal: true

module Components
  module People
    # Person form view component for new/edit actions
    class FormView < Components::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::TurboFrameTag

      attr_reader :person, :title, :subtitle

      def initialize(person:, title: nil, subtitle: nil)
        @person = person
        @title = title || default_title
        @subtitle = subtitle || default_subtitle
        super()
      end

      def view_template
        turbo_frame_tag 'modal' do
          div(class: 'container mx-auto px-4 py-8 max-w-2xl') do
            render_header
            render_form
          end
        end
      end

      private

      def default_title
        person.new_record? ? 'New Person' : 'Edit Person'
      end

      def default_subtitle
        person.new_record? ? 'Add a new person to track medications for' : "Update #{person.name}'s details"
      end

      def render_header
        div(class: 'mb-8') do
          p(class: 'text-sm font-medium uppercase tracking-wide text-slate-500 mb-2') { subtitle }
          h1(class: 'text-4xl font-bold text-slate-900') { title }
        end
      end

      def render_form
        form_with(model: person, id: 'person_form', class: 'space-y-6', data: { turbo_frame: '_top' }) do |f|
          render_errors if person.errors.any?
          render_form_fields(f)
          render_actions(f)
        end
      end

      def render_errors
        Alert(variant: :destructive, class: 'mb-6') do
          AlertTitle { "#{person.errors.count} error(s) prohibited this person from being saved:" }
          AlertDescription do
            ul(class: 'list-disc list-inside space-y-1') do
              person.errors.full_messages.each do |message|
                li { message }
              end
            end
          end
        end
      end

      def render_form_fields(f)
        div(class: 'space-y-6') do
          render_name_field(f)
          render_email_field(f)
          render_date_of_birth_field(f)
          render_person_type_field(f)
          render_capacity_field(f)
        end
      end

      def render_name_field(f)
        FormField do
          FormFieldLabel(for: 'person_name') { 'Name' }
          render f.text_field(
            :name,
            class: input_classes,
            required: true
          )
        end
      end

      def render_email_field(f)
        FormField do
          FormFieldLabel(for: 'person_email') { 'Email' }
          render f.email_field(
            :email,
            class: input_classes
          )
        end
      end

      def render_date_of_birth_field(f)
        FormField do
          FormFieldLabel(for: 'person_date_of_birth') { 'Date of Birth' }
          render f.date_field(
            :date_of_birth,
            class: input_classes,
            required: true
          )
        end
      end

      def render_person_type_field(_f)
        FormField do
          FormFieldLabel(for: 'person_person_type') { 'Person Type' }
          Select do
            SelectInput(
              name: 'person[person_type]',
              id: 'person_person_type',
              value: person.person_type
            )
            SelectTrigger do
              SelectValue(placeholder: 'Select person type') do
                person.person_type&.humanize || 'Select person type'
              end
            end
            SelectContent do
              Person.person_types.keys.each do |type|
                SelectItem(value: type) { type.humanize }
              end
            end
          end
        end
      end

      def render_capacity_field(f)
        FormField do
          div(class: 'flex items-center gap-3') do
            render f.check_box(
              :has_capacity,
              class: 'h-4 w-4 rounded border-border text-primary focus:ring-ring',
              data: {
                controller: 'capacity-hint',
                action: 'capacity-hint#toggleHint',
                capacity_hint_target: 'checkbox'
              }
            )
            FormFieldLabel(for: 'person_has_capacity', class: 'mb-0') do
              'Has capacity to manage own medication'
            end
          end
          render_capacity_hint
        end
      end

      def render_capacity_hint
        hint_visible = !person.has_capacity
        div(
          class: 'mt-2 text-sm text-amber-600',
          data: { capacity_hint_target: 'hint' },
          style: hint_visible ? '' : 'display: none;'
        ) do
          strong { 'Note: ' }
          plain 'A person without capacity must have at least one carer assigned.'
          if person.persisted? && person.carers.empty?
            plain ' Please assign a carer before removing capacity.'
          end
        end
      end

      def render_actions(_f)
        div(class: 'flex gap-3 justify-end pt-4') do
          Link(href: people_path, variant: :outline) { 'Cancel' }
          Button(type: :submit, variant: :primary) do
            person.new_record? ? 'Create Person' : 'Update Person'
          end
        end
      end

      def input_classes
        'flex h-9 w-full rounded-md border bg-background px-3 py-1 text-sm shadow-sm ' \
          'border-border placeholder:text-muted-foreground focus-visible:outline-none ' \
          'focus-visible:ring-1 focus-visible:ring-ring'
      end
    end
  end
end
