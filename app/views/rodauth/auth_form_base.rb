# frozen_string_literal: true

module Views
  module Rodauth
    class AuthFormBase < Views::Rodauth::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::LinkTo

      def view_template
        page_layout do
          render_page_header(title: 'MedTracker', subtitle: page_subtitle)
          form_section
        end
      end

      private

      def page_subtitle
        raise NotImplementedError
      end

      def card_title
        raise NotImplementedError
      end

      def card_description
        raise NotImplementedError
      end

      def flash_id
        raise NotImplementedError
      end

      def flash_section
        return if flash_message.blank?

        div(id: flash_id) do
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
        render RubyUI::Card.new(class: "#{card_classes} overflow-hidden") do
          render_card_header
          render_card_content
        end
      end

      def render_card_header
        render RubyUI::CardHeader.new(class: 'space-y-2 bg-white/60') do
          render RubyUI::CardTitle.new(class: 'text-2xl font-semibold text-slate-900') { card_title }
          render RubyUI::CardDescription.new(class: 'text-base text-slate-600') do
            plain card_description
          end
        end
      end

      def render_card_content
        render RubyUI::CardContent.new(class: 'space-y-6 p-6 sm:p-8') do
          flash_section
          render_form
          render_other_options
        end
      end

      def render_form
        raise NotImplementedError
      end

      def render_other_options
        # Override in subclasses if needed
      end

      def render_form_field(label:, input_attrs:, error: nil)
        render RubyUI::FormField.new do
          render RubyUI::FormFieldLabel.new(for: input_attrs[:id]) { label }
          render RubyUI::Input.new(**input_attrs)

          p(class: 'text-sm text-red-600 mt-1') { error } if error.present?
        end
      end

      def submit_button(label)
        render RubyUI::Button.new(type: :submit, variant: :primary, size: :md, class: 'w-full') { label }
      end
    end
  end
end
