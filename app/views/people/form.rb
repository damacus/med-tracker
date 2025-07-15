# frozen_string_literal: true

module Views
  module People
    # Form component for creating and editing people
    class Form < Views::Base
      def initialize(person:)
        super()
        @person = person
      end

      def view_template
        form_with(model: @person, id: 'person_form', data: { turbo_frame: '_top' }) do |form|
          render_errors(form)
          
          div do
            form.label :name, style: 'display: block'
            form.text_field :name
          end

          div do
            form.label :email, style: 'display: block'
            form.text_field :email
          end

          div do
            form.label :date_of_birth, style: 'display: block'
            form.date_field :date_of_birth
          end

          div(class: 'form__actions') do
            form.submit class: 'button button--primary'
            link_to 'Cancel', people_path, class: 'button button--secondary'
          end
        end
      end

      private

      def render_errors(_form)
        return unless @person.errors.any?

        div(style: 'color: red') do
          h2 { "#{pluralize(@person.errors.count, 'error')} prohibited this person from being saved:" }

          ul do
            @person.errors.full_messages.each do |message|
              li { message }
            end
          end
        end
      end
    end
  end
end
