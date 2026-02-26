# frozen_string_literal: true

module Views
  module Rodauth
    class VerifyAccount < Views::Rodauth::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::LinkTo

      def view_template
        page_layout do
          render_page_header(
            title: t('app.name'),
            subtitle: t('rodauth.views.verify_account.page_subtitle')
          )
          form_section
        end
      end

      private

      def flash_section
        return if flash_message.blank?

        div(id: 'verify-flash') do
          render RubyUI::Alert.new(variant: flash_variant) do
            plain(flash_message)
          end
        end
      end

      def flash_message
        view_context.flash[:alert] || view_context.flash[:notice]
      end

      def flash_variant
        view_context.flash[:alert].present? ? :destructive : :success
      end

      def form_section
        render RubyUI::Card.new(class: card_classes) do
          render_card_header
          render_card_content
        end
      end

      def render_card_header
        render RubyUI::CardHeader.new(class: 'space-y-2 bg-white/60') do
          render RubyUI::CardTitle.new(class: 'text-2xl font-semibold text-slate-900') { t('rodauth.views.verify_account.card_title') }
          render RubyUI::CardDescription.new(class: 'text-base text-slate-600') do
            plain t('rodauth.views.verify_account.card_description')
          end
        end
      end

      def render_card_content
        render RubyUI::CardContent.new(class: 'space-y-6 p-6 sm:p-8') do
          flash_section
          render_verify_form
        end
      end

      def render_verify_form
        render RubyUI::Form.new(action: view_context.rodauth.verify_account_path, method: :post, class: 'space-y-6', data_turbo: 'false') do
          authenticity_token_field
          key_field
          submit_button
        end
      end

      def key_field
        input(type: 'hidden', name: 'key', value: view_context.params[:key])
      end

      def submit_button
        render RubyUI::Button.new(type: :submit, variant: :primary, size: :md, class: 'w-full') do
          t('rodauth.views.verify_account.submit')
        end
      end
    end
  end
end
