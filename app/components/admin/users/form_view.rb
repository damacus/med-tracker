# frozen_string_literal: true

module Components
  module Admin
    module Users
      class FormView < Components::Base
        include Phlex::Rails::Helpers::FormWith
        include Phlex::Rails::Helpers::Pluralize

        attr_reader :user, :url_helpers

        def initialize(user:, url_helpers:)
          @user = user
          @url_helpers = url_helpers
          super()
        end

        def view_template
          div(class: 'container mx-auto px-4 py-8 max-w-2xl') do
            render_header
            render_form
          end
        end

        private

        def render_header
          div(class: 'mb-8') do
            Heading(level: 1, class: 'mb-2') { form_title }
            Text(weight: 'muted') { 'Fill in the details below to create a new user account.' }
          end
        end

        def form_title
          user.new_record? ? 'Create New User' : 'Edit User'
        end

        def render_form
          form_with(
            model: [:admin, user],
            class: 'space-y-6',
            data: { testid: 'user-form' }
          ) do |form|
            render_errors if user.errors.any?
            render_form_fields(form)
            render_form_actions
          end
        end

        def render_errors
          render RubyUI::Alert.new(variant: :destructive, class: 'mb-6') do
            div do
              Heading(level: 2, size: '3', class: 'font-semibold mb-2') do
                plain "#{pluralize(user.errors.count, 'error')} prevented this user from being saved:"
              end
              ul(class: 'my-2 ml-6 list-disc [&>li]:mt-1') do
                user.errors.full_messages.each do |message|
                  li { message }
                end
              end
            end
          end
        end

        def render_form_fields(form)
          Card do
            CardContent(class: 'pt-6 space-y-6') do
              render_person_fields(form)
              render_email_field(form)
              render_password_fields(form)
              render_role_field(form)
            end
          end
        end

        def render_person_fields(form)
          form.fields_for :person do |person_form|
            div(class: 'space-y-4') do
              render_name_field(person_form)
              render_date_of_birth_field(person_form)
            end
          end
        end

        def render_name_field(_person_form)
          render RubyUI::FormField.new do
            render RubyUI::FormFieldLabel.new(for: 'user_person_attributes_name') { 'Name' }
            render RubyUI::Input.new(
              type: :text,
              name: 'user[person_attributes][name]',
              id: 'user_person_attributes_name',
              value: user.person&.name,
              required: true
            )
          end
        end

        def render_date_of_birth_field(_person_form)
          render RubyUI::FormField.new do
            render RubyUI::FormFieldLabel.new(for: 'user_person_attributes_date_of_birth') { 'Date of birth' }
            render RubyUI::Input.new(
              type: :date,
              name: 'user[person_attributes][date_of_birth]',
              id: 'user_person_attributes_date_of_birth',
              value: user.person&.date_of_birth&.to_s,
              required: true
            )
          end
        end

        def render_email_field(_form)
          render RubyUI::FormField.new do
            render RubyUI::FormFieldLabel.new(for: 'user_email_address') { 'Email address' }
            render RubyUI::Input.new(
              type: :email,
              name: 'user[email_address]',
              id: 'user_email_address',
              value: user.email_address,
              required: true
            )
          end
        end

        def render_password_fields(_form)
          div(class: 'space-y-4') do
            render_password_field
            render_password_confirmation_field
          end
        end

        def render_password_field
          render RubyUI::FormField.new do
            render RubyUI::FormFieldLabel.new(for: 'user_password') { 'Password' }
            render RubyUI::Input.new(
              type: :password,
              name: 'user[password]',
              id: 'user_password',
              required: user.new_record?
            )
          end
        end

        def render_password_confirmation_field
          render RubyUI::FormField.new do
            render RubyUI::FormFieldLabel.new(for: 'user_password_confirmation') { 'Password confirmation' }
            render RubyUI::Input.new(
              type: :password,
              name: 'user[password_confirmation]',
              id: 'user_password_confirmation',
              required: user.new_record?
            )
          end
        end

        def render_role_field(_form)
          render RubyUI::FormField.new do
            render RubyUI::FormFieldLabel.new(for: 'user_role') { 'Role' }
            select(
              name: 'user[role]',
              id: 'user_role',
              class: input_classes,
              required: true
            ) do
              option(value: '', selected: user.role.blank?) { 'Select role' }
              User.roles.each_key do |role|
                option(value: role, selected: user.role == role) { role.titleize }
              end
            end
          end
        end

        def render_form_actions
          div(class: 'flex items-center justify-between pt-6') do
            Link(href: url_helpers.admin_users_path, variant: :outline) { 'Cancel' }
            Button(type: :submit, variant: :primary) do
              user.new_record? ? 'Create User' : 'Update User'
            end
          end
        end

        def input_classes
          'w-full rounded-md border border-input bg-background px-3 py-2 text-sm ' \
            'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring'
        end

        def button_secondary_classes
          'inline-flex items-center justify-center rounded-md font-medium transition-colors ' \
            'px-4 py-2 h-10 text-sm border border-input bg-background hover:bg-accent hover:text-accent-foreground'
        end
      end
    end
  end
end
