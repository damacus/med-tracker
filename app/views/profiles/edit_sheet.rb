# frozen_string_literal: true

module Views
  module Profiles
    class EditSheet < Views::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::Routes

      attr_reader :person, :account

      def initialize(person:, account:, field_config:)
        super()
        @person = person
        @account = account
        @field = field_config[:field]
        @label = field_config[:label]
        @current_value = field_config[:current_value]
        @button_text = field_config.fetch(:button_text, 'Edit')
      end

      def view_template
        render Sheet.new do
          render_sheet_trigger
          render_sheet_content
        end
      end

      private

      def render_sheet_trigger
        render SheetTrigger.new do
          render Button.new(variant: :outline, size: :sm) { @button_text }
        end
      end

      def render_sheet_content
        render SheetContent.new(class: 'sm:max-w-sm') do
          render_sheet_header
          render_sheet_form
        end
      end

      def render_sheet_header
        render SheetHeader.new do
          render(SheetTitle.new { "Edit #{@label}" })
          render(SheetDescription.new { "Update your #{@label.downcase} information." })
        end
      end

      def render_sheet_form
        form_with(model: @field == :email ? @account : @person, url: profile_path, method: :patch) do
          render SheetMiddle.new do
            render_field_input
          end
          render_sheet_footer
        end
      end

      def render_sheet_footer
        render SheetFooter.new do
          render Button.new(variant: :outline, data: { action: 'click->ruby-ui--sheet-content#close' }) { 'Cancel' }
          render Button.new(type: :submit) { 'Save' }
        end
      end

      def render_field_input
        case @field
        when :email
          label(class: 'text-sm font-medium text-slate-900 mb-2 block') { 'Email Address' }
          render Input.new(
            type: 'email',
            name: 'account[email]',
            placeholder: 'your.email@example.com',
            value: @current_value
          )
        when :date_of_birth
          label(class: 'text-sm font-medium text-slate-900 mb-2 block') { 'Date of Birth' }
          render Input.new(
            type: 'date',
            name: 'person[date_of_birth]',
            value: @person.date_of_birth&.strftime('%Y-%m-%d')
          )
        end
      end
    end
  end
end
