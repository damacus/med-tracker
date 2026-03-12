# frozen_string_literal: true

module Components
  module People
    # Person form view component for new/edit actions
    class FormView < Components::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::TurboFrameTag
      include RubyUI

      attr_reader :person, :title, :subtitle, :return_to, :is_modal, :assigned_location

      def initialize(person:, is_modal: false, assigned_location: nil, **options)
        @person = person
        @title = options[:title] || default_title
        @subtitle = options[:subtitle] || default_subtitle
        @return_to = options[:return_to]
        @is_modal = is_modal
        @assigned_location = assigned_location
        super()
      end

      def view_template
        if is_modal
          render_form
        else
          div(class: 'container mx-auto px-4 py-12 max-w-2xl') do
            Card(class: 'overflow-hidden border-none shadow-2xl rounded-[2.5rem] bg-white') do
              div(class: 'p-10') do
                render_header_section
                render_form
              end
            end
          end
        end
      end

      private

      def render_header_section
        div(class: 'mb-8 space-y-2') do
          Heading(level: 1, size: '7', class: 'font-black tracking-tight') { title }
          Text(weight: 'muted') { subtitle }
        end
      end

      def default_title
        person.new_record? ? 'New Person' : 'Edit Person'
      end

      def default_subtitle
        person.new_record? ? 'Add a new person to track medications for' : "Update #{person.name}'s details"
      end

      def render_header
        # Header is rendered by Modal component
      end

      def render_form
        form_with(model: person, id: 'person_form', class: 'space-y-6') do |f|
          render_errors if person.errors.any?
          input(type: 'hidden', name: 'return_to', value: return_to) if return_to.present?
          render_location_hint
          render_form_fields(f)
          render_actions(f)
        end
      end

      def render_location_hint
        return unless person.new_record?
        return if assigned_location.blank?

        Alert(class: 'border-primary/20 bg-primary/5 text-primary') do
          AlertTitle { 'Location' }
          AlertDescription do
            plain "This person will be created at #{assigned_location.name}."
          end
        end
      end

      def render_errors
        Alert(variant: :destructive, class: 'mb-6') do
          AlertTitle { "#{person.errors.count} error(s) prohibited this person from being saved:" }
          AlertDescription do
            ul(class: 'my-2 ml-6 list-disc [&>li]:mt-1') do
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

      def render_name_field(_f)
        FormField do
          FormFieldLabel(for: 'person_name') { 'Name' }
          Input(
            type: :text,
            name: 'person[name]',
            id: 'person_name',
            value: person.name,
            required: true,
            placeholder: t('forms.people.name_placeholder', default: 'e.g., Jane Doe'),
            class: field_error_class(person, :name),
            **field_error_attributes(person, :name, input_id: 'person_name')
          )
          render_field_error(person, :name, input_id: 'person_name')
        end
      end

      def render_email_field(_f)
        FormField do
          FormFieldLabel(for: 'person_email') { 'Email' }
          Input(
            type: :email,
            name: 'person[email]',
            id: 'person_email',
            value: person.email,
            placeholder: t('forms.people.email_placeholder', default: 'e.g., jane@example.com'),
            class: field_error_class(person, :email),
            **field_error_attributes(person, :email, input_id: 'person_email')
          )
          render_field_error(person, :email, input_id: 'person_email')
        end
      end

      def render_date_of_birth_field(_f)
        FormField do
          FormFieldLabel(for: 'person_date_of_birth') { 'Date of Birth' }
          Input(
            type: :date,
            name: 'person[date_of_birth]',
            id: 'person_date_of_birth',
            value: person.date_of_birth&.to_s,
            required: true,
            class: field_error_class(person, :date_of_birth),
            **field_error_attributes(person, :date_of_birth, input_id: 'person_date_of_birth')
          )
          render_field_error(person, :date_of_birth, input_id: 'person_date_of_birth')
        end
      end

      def render_person_type_field(_f)
        FormField do
          FormFieldLabel(for: 'person_person_type') { 'Person Type' }
          Select do
            SelectInput(
              name: 'person[person_type]',
              id: 'person_person_type',
              value: selected_person_type
            )
            SelectTrigger do
              SelectValue(placeholder: 'Select person type') do
                selected_person_type&.humanize || 'Select person type'
              end
            end
            SelectContent do
              available_person_types.each do |type|
                SelectItem(value: type) { type.humanize }
              end
            end
          end
        end
      end

      def render_capacity_field(_f)
        FormField do
          div(class: 'flex items-center gap-3') do
            input(
              type: 'hidden',
              name: 'person[has_capacity]',
              value: '0'
            )
            input(
              type: 'checkbox',
              name: 'person[has_capacity]',
              id: 'person_has_capacity',
              value: '1',
              checked: person.has_capacity,
              class: checkbox_classes,
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
          class: ['mt-2 text-sm text-amber-600', ('hidden' unless hint_visible)],
          data: { capacity_hint_target: 'hint' }
        ) do
          strong { 'Note: ' }
          plain 'A person without capacity must have at least one carer assigned.'
          plain ' Please assign a carer before removing capacity.' if person.persisted? && person.carers.empty?
        end
      end

      def render_actions(_f)
        div(class: 'flex gap-3 justify-end pt-4') do
          Button(variant: :ghost, data: { action: 'click->ruby-ui--dialog#dismiss' }) { 'Cancel' }
          Button(type: :submit, variant: :primary) do
            person.new_record? ? 'Create Person' : 'Update Person'
          end
        end
      end

      def available_person_types
        @available_person_types ||= begin
          allowed_types = Person.person_types.keys.select do |type|
            view_context.policy(Person.new(person_type: type)).create?
          end
          allowed_types.presence || Person.person_types.keys
        end
      end

      def selected_person_type
        return person.person_type if available_person_types.include?(person.person_type)
        return available_person_types.first if person.new_record?

        person.person_type
      end
    end
  end
end
