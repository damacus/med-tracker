# frozen_string_literal: true

module Views
  module Profiles
    class Show < Views::Base
      include Phlex::Rails::Helpers::LinkTo
      include Phlex::Rails::Helpers::Routes
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::ButtonTo

      attr_reader :person, :account

      def initialize(person:, account:)
        super()
        @person = person
        @account = account
      end

      def view_template
        div(class: 'container mx-auto px-4 py-8 pb-24 md:pb-8 max-w-4xl') do
          render_header
          render_profile_sections
        end
      end

      private

      def render_header
        div(class: 'mb-8') do
          h1(class: 'text-3xl font-bold text-foreground') { 'My Profile' }
          p(class: 'mt-2 text-muted-foreground') do
            'Manage your personal information and account settings'
          end
        end
      end

      def render_profile_sections
        div(class: 'space-y-6') do
          render_theme_picker_card
          render_personal_info_card
          render_account_security_card
          render_two_factor_card
          render_notifications_card
          render_danger_zone_card
        end
      end

      def render_theme_picker_card
        render ThemePickerCard.new
      end

      def render_personal_info_card
        render Card.new do
          render_personal_info_header
          render_personal_info_content
        end
      end

      def render_personal_info_header
        render CardHeader.new do
          render(CardTitle.new { 'Personal Information' })
          render(CardDescription.new { 'Your basic profile information' })
        end
      end

      def render_personal_info_content
        render PersonalInfoContent.new(person: person, account: account)
      end

      def render_account_security_card
        render Card.new do
          render CardHeader.new do
            render(CardTitle.new { 'Account Security' })
            render(CardDescription.new do
              'Manage your login credentials and security settings'
            end)
          end
          render CardContent.new(class: 'space-y-3') do
            render_email_change_sheet
            render_password_change_sheet
          end
        end
      end

      def render_danger_zone_card
        render Card.new(class: 'border-destructive') do
          render CardHeader.new do
            render CardTitle.new(class: 'text-destructive') { 'Danger Zone' }
            render(CardDescription.new do
              'Irreversible account actions'
            end)
          end
          render CardContent.new do
            render_close_account_dialog
          end
        end
      end

      def render_info_row(label, value)
        div(class: 'flex items-center justify-between border-b border-border py-3 last:border-0') do
          dt(class: 'text-sm font-medium text-muted-foreground') { label }
          dd(class: 'text-sm text-foreground') { value || 'Not set' }
        end
      end

      def render_action_button(title, href, description, variant: :outline, button_text: 'Update')
        div(class: 'flex items-start justify-between rounded-lg border border-border bg-card/70 p-4 transition-colors hover:bg-accent/50') do
          div(class: 'flex-1') do
            h3(class: 'text-sm font-medium text-foreground') { title }
            p(class: 'mt-1 text-sm text-muted-foreground') { description }
          end
          div(class: 'ml-4') do
            render RubyUI::Link.new(
              variant: variant,
              size: :sm,
              href: href
            ) { button_text }
          end
        end
      end

      def render_info_row_with_edit(label, value, field)
        div(class: 'flex items-center justify-between border-b border-border py-3 last:border-0') do
          dt(class: 'text-sm font-medium text-muted-foreground') { label }
          div(class: 'flex items-center gap-2') do
            dd(class: 'text-sm text-foreground') { value || 'Not set' }
            render_edit_sheet(label, field, value)
          end
        end
      end

      def render_edit_sheet(label, field, current_value, button_text: 'Edit')
        render EditSheet.new(
          person: person,
          account: account,
          field_config: {
            label: label,
            field: field,
            current_value: current_value,
            button_text: button_text
          }
        )
      end

      def render_email_change_sheet
        div(class: 'flex items-start justify-between rounded-lg border border-border bg-card/70 p-4 transition-colors hover:bg-accent/50') do
          div(class: 'flex-1') do
            h3(class: 'text-sm font-medium text-foreground') { 'Change Email Address' }
            p(class: 'mt-1 text-sm text-muted-foreground') { 'Update the email address you use to sign in' }
          end
          div(class: 'ml-4') do
            render RubyUI::Link.new(
              variant: :outline,
              size: :sm,
              href: '/change-login',
              data: { turbo_frame: 'modal' }
            ) { 'Change' }
          end
        end
      end

      def render_password_change_sheet
        div(class: 'flex items-start justify-between rounded-lg border border-border bg-card/70 p-4 transition-colors hover:bg-accent/50') do
          div(class: 'flex-1') do
            h3(class: 'text-sm font-medium text-foreground') { 'Change Password' }
            p(class: 'mt-1 text-sm text-muted-foreground') { 'Update your password to keep your account secure' }
          end
          div(class: 'ml-4') do
            render RubyUI::Link.new(
              variant: :outline,
              size: :sm,
              href: '/change-password',
              data: { turbo_frame: 'modal' }
            ) { 'Change' }
          end
        end
      end

      def render_password_sheet
        render PasswordSheet.new
      end

      def render_close_account_dialog
        render CloseAccountDialog.new
      end

      def render_two_factor_card
        render TwoFactorCard.new(account: account)
      end

      def render_notifications_card
        render NotificationsCard.new(person: person)
      end
    end
  end
end
