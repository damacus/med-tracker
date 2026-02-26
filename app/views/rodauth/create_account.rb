# frozen_string_literal: true

module Views
  module Rodauth
    class CreateAccount < Views::Rodauth::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::LinkTo

      def view_template
        page_layout do
          render_page_header(
            title: t('app.name'),
            subtitle: t('rodauth.views.create_account.page_subtitle')
          )
          form_section
        end
      end

      private

      def flash_section
        return if flash_message.blank?

        div(id: 'signup-flash') do
          render RubyUI::Alert.new(variant: flash_variant) do
            plain(flash_message)
          end
        end
      end

      def flash_message
        view_context.flash[:alert] || view_context.flash[:notice] || rodauth_error
      end

      def rodauth_error
        view_context.rodauth.field_error('login') ||
          view_context.rodauth.field_error('password') ||
          view_context.rodauth.field_error('password-confirm')
      end

      def flash_variant
        view_context.flash[:alert].present? || rodauth_error.present? ? :destructive : :success
      end

      def form_section
        render RubyUI::Card.new(class: card_classes) do
          render_card_header
          render_card_content
        end
      end

      def render_card_header
        render RubyUI::CardHeader.new(class: 'space-y-2 bg-white/60') do
          render RubyUI::CardTitle.new(class: 'text-2xl font-semibold text-slate-900') { t('rodauth.views.create_account.card_title') }
          render RubyUI::CardDescription.new(class: 'text-base text-slate-600') do
            plain t('rodauth.views.create_account.card_description')
          end
        end
      end

      def render_card_content
        render RubyUI::CardContent.new(class: 'space-y-6 p-6 sm:p-8') do
          flash_section
          render_signup_form
          render_other_options
        end
      end

      def render_signup_form
        render RubyUI::Form.new(action: view_context.rodauth.create_account_path, method: :post, class: 'space-y-6', data_turbo: 'false') do
          authenticity_token_field
          name_field
          date_of_birth_field
          email_field
          password_field
          password_confirm_field
          submit_button
        end
      end

      def name_field
        render_form_field(
          label: t('rodauth.views.create_account.name_label'),
          input_attrs: {
            type: :text,
            name: 'name',
            id: 'name',
            required: true,
            autofocus: true,
            autocomplete: 'name',
            placeholder: t('rodauth.views.create_account.name_placeholder'),
            value: view_context.params[:name]
          },
          error: view_context.rodauth.field_error('name')
        )
      end

      def date_of_birth_field
        render_form_field(
          label: t('rodauth.views.create_account.date_of_birth_label'),
          input_attrs: {
            type: :date,
            name: 'date_of_birth',
            id: 'date_of_birth',
            required: true,
            autocomplete: 'bday',
            value: view_context.params[:date_of_birth]
          },
          error: view_context.rodauth.field_error('date_of_birth')
        )
      end

      def email_field
        render_form_field(
          label: t('rodauth.views.create_account.email_label'),
          input_attrs: {
            type: :email,
            name: 'email',
            id: 'email',
            required: true,
            autocomplete: 'email',
            placeholder: t('rodauth.views.create_account.email_placeholder'),
            value: view_context.params[:email]
          },
          error: view_context.rodauth.field_error('login')
        )
      end

      def password_field
        render_form_field(
          label: t('rodauth.views.create_account.password_label'),
          input_attrs: {
            type: :password,
            name: 'password',
            id: 'password',
            required: true,
            autocomplete: 'new-password',
            placeholder: t('rodauth.views.create_account.password_placeholder'),
            minlength: 12,
            maxlength: 72
          },
          error: view_context.rodauth.field_error('password')
        )
      end

      def password_confirm_field
        render_form_field(
          label: t('rodauth.views.create_account.confirm_password_label'),
          input_attrs: {
            type: :password,
            name: 'password-confirm',
            id: 'password-confirm',
            required: true,
            autocomplete: 'new-password',
            placeholder: t('rodauth.views.create_account.confirm_password_placeholder'),
            minlength: 12,
            maxlength: 72
          },
          error: view_context.rodauth.field_error('password-confirm')
        )
      end

      def render_form_field(label:, input_attrs:, error: nil)
        render RubyUI::FormField.new do
          render RubyUI::FormFieldLabel.new(for: input_attrs[:id]) { label }
          render RubyUI::Input.new(**input_attrs)

          p(class: 'text-sm text-red-600 mt-1') { error } if error.present?
        end
      end

      def submit_button
        render RubyUI::Button.new(type: :submit, variant: :primary, size: :md, class: 'w-full') { t('rodauth.views.create_account.submit') }
      end

      def render_other_options
        div(class: 'space-y-3 border-t border-slate-200 pt-6') do
          h3(class: 'text-sm font-medium text-slate-700') { t('rodauth.views.create_account.existing_account') }
          div(class: 'flex flex-col gap-2 text-sm') do
            render RubyUI::Link.new(href: view_context.rodauth.login_path, variant: :link) do
              t('rodauth.views.create_account.sign_in')
            end
          end
        end
      end
    end
  end
end
