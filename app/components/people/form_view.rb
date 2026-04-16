# frozen_string_literal: true

module Components
  module People
    # Person form view component for new/edit actions
    class FormView < Components::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::TurboFrameTag
      include RubyUI

      attr_reader :person, :return_to, :is_modal, :assigned_location

      def initialize(person:, is_modal: false, assigned_location: nil, **options)
        @person = person
        @explicit_title = options[:title]
        @explicit_subtitle = options[:subtitle]
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
            m3_card(variant: :elevated, class: 'overflow-visible border-none shadow-elevation-3 rounded-[2.5rem]') do
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
        div(class: 'mb-8 space-y-2 text-center md:text-left') do
          m3_heading(variant: :display_small, level: 1, class: 'font-black tracking-tight') { title }
          m3_text(variant: :body_large, class: 'text-on-surface-variant font-medium') { subtitle }
        end
      end

      def title
        @explicit_title || default_title
      end

      def subtitle
        @explicit_subtitle || default_subtitle
      end

      def default_title
        if person.new_record?
          t('people.form.new_heading')
        else
          t('people.form.edit_heading')
        end
      end

      def default_subtitle
        if person.new_record?
          t('people.form.new_subheading')
        else
          t('people.form.edit_subheading', name: person.name)
        end
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
          AlertTitle { t('people.form.location_title') }
          AlertDescription do
            plain t('people.form.location_description', location: assigned_location.name)
          end
        end
      end

      def render_errors
        render RubyUI::Alert.new(variant: :destructive,
                                 class: 'mb-6 rounded-shape-xl border-none shadow-elevation-1') do
          div(class: 'flex items-start gap-3') do
            render Icons::AlertCircle.new(size: 20)
            div do
              m3_heading(variant: :title_medium, level: 2, class: 'font-bold mb-1') do
                plain "#{person.errors.count} error(s) prohibited this person from being saved:"
              end
              ul(class: 'text-sm opacity-90 list-disc pl-4 space-y-1 font-medium') do
                person.errors.full_messages.each do |message|
                  li { message }
                end
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
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(
            for: 'person_name',
            class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant px-1'
          ) { t('people.form.name') }
          m3_input(
            type: :text,
            name: 'person[name]',
            id: 'person_name',
            value: person.name,
            required: true,
            placeholder: t('forms.people.name_placeholder', default: 'e.g., Jane Doe'),
            class: 'rounded-md border-outline-variant bg-surface-container-lowest py-4 px-4 ' \
                   "#{field_error_class(person, :name)}",
            **field_error_attributes(person, :name, input_id: 'person_name')
          )
          render_field_error(person, :name, input_id: 'person_name')
        end
      end

      def render_email_field(_f)
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(
            for: 'person_email',
            class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant px-1'
          ) { t('people.form.email') }
          m3_input(
            type: :email,
            name: 'person[email]',
            id: 'person_email',
            value: person.email,
            placeholder: t('forms.people.email_placeholder', default: 'e.g., jane@example.com'),
            class: 'rounded-md border-outline-variant bg-surface-container-lowest py-4 px-4 ' \
                   "#{field_error_class(person, :email)}",
            **field_error_attributes(person, :email, input_id: 'person_email')
          )
          render_field_error(person, :email, input_id: 'person_email')
        end
      end

      def render_date_of_birth_field(_f)
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(
            for: 'person_date_of_birth',
            class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant px-1'
          ) { t('people.form.date_of_birth') }
          m3_input(
            type: :string,
            name: 'person[date_of_birth]',
            id: 'person_date_of_birth',
            value: person.date_of_birth&.to_fs(:db),
            required: true,
            placeholder: 'YYYY-MM-DD',
            class: 'rounded-md border-outline-variant bg-surface-container-lowest py-4 px-4 ' \
                   'transition-all ' \
                   "#{field_error_class(person, :date_of_birth)}",
            data: {
              controller: 'ruby-ui--calendar-input'
            },
            **field_error_attributes(person, :date_of_birth, input_id: 'person_date_of_birth')
          )
          render RubyUI::Calendar.new(
            input_id: '#person_date_of_birth',
            date_format: 'yyyy-MM-dd',
            class: 'rounded-md border shadow-elevation-2 bg-surface-container-high'
          )
          render_field_error(person, :date_of_birth, input_id: 'person_date_of_birth')
        end
      end

      def render_person_type_field(_f)
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(
            for: 'person_person_type_trigger',
            class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant px-1'
          ) { t('people.form.person_type') }
          render RubyUI::Combobox.new(class: 'w-full') do
            render RubyUI::ComboboxTrigger.new(
              placeholder: selected_person_type&.humanize || t('people.form.select_person_type'),
              class: 'rounded-md border-outline-variant bg-surface-container-lowest py-4 px-4 transition-all'
            )

            render RubyUI::ComboboxPopover.new do
              render RubyUI::ComboboxList.new do
                available_person_types.each do |type|
                  render RubyUI::ComboboxItem.new do
                    render RubyUI::ComboboxRadio.new(
                      name: 'person[person_type]',
                      id: "person_person_type_#{type}",
                      value: type,
                      checked: selected_person_type == type
                    )
                    span { type.humanize }
                  end
                end
              end
            end
          end
        end
      end

      def render_capacity_field(_f)
        div(class: 'space-y-2') do
          div(
            class: 'flex items-center gap-3 p-4 rounded-xl border border-outline-variant ' \
                   'bg-surface-container-low state-layer relative'
          ) do
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
              class: "z-10 #{checkbox_classes}",
              data: {
                controller: 'capacity-hint',
                action: 'capacity-hint#toggleHint',
                capacity_hint_target: 'checkbox'
              }
            )
            render RubyUI::FormFieldLabel.new(for: 'person_has_capacity',
                                              class: 'mb-0 z-10 font-bold text-foreground cursor-pointer') do
              t('people.form.has_capacity')
            end
          end
          render_capacity_hint
        end
      end

      def render_capacity_hint
        hint_visible = !person.has_capacity
        div(
          class: ['mt-2 text-sm text-on-warning-container font-medium px-1', ('hidden' unless hint_visible)],
          data: { capacity_hint_target: 'hint' }
        ) do
          strong { "#{t('people.form.note')}: " }
          plain t('people.form.capacity_hint')
          plain " #{t('people.form.capacity_hint_assign_carer')}" if person.persisted? && person.carers.empty?
        end
      end

      def render_actions(_f)
        div(class: 'flex gap-3 justify-end pt-4') do
          m3_button(variant: :text, data: { action: 'click->ruby-ui--dialog#dismiss' }) { t('people.form.cancel') }
          m3_button(type: :submit, variant: :filled) do
            person.new_record? ? t('people.form.create') : t('people.form.update')
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
