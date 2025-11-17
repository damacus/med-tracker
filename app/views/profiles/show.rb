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
        div(class: 'container mx-auto px-4 py-8 max-w-4xl') do
          render_header
          render_profile_sections
        end
      end

      private

      def render_header
        div(class: 'mb-8') do
          h1(class: 'text-3xl font-bold text-slate-900') { 'My Profile' }
          p(class: 'text-slate-600 mt-2') do
            'Manage your personal information and account settings'
          end
        end
      end

      def render_profile_sections
        div(class: 'space-y-6') do
          render_personal_info_card
          render_account_security_card
          render_danger_zone_card
        end
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
        render CardContent.new(class: 'space-y-4') do
          render_info_row('Name', person.name)
          render_info_row('Email', account.email)
          render_info_row('Date of Birth', formatted_date_of_birth)
          render_info_row('Age', person.age.to_s) if person.age
          render_info_row('Person Type', person.person_type.humanize)
          render_info_row('Has Capacity', person.has_capacity? ? 'Yes' : 'No')
        end
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
        div(class: 'flex justify-between items-center py-3 border-b border-slate-200 last:border-0') do
          dt(class: 'text-sm font-medium text-slate-600') { label }
          dd(class: 'text-sm text-slate-900') { value || 'Not set' }
        end
      end

      def render_action_button(title, href, description, variant: :outline, button_text: 'Update')
        div(class: 'flex items-start justify-between p-4 border border-slate-200 rounded-lg hover:bg-slate-50 transition-colors') do
          div(class: 'flex-1') do
            h3(class: 'text-sm font-medium text-slate-900') { title }
            p(class: 'text-sm text-slate-600 mt-1') { description }
          end
          div(class: 'ml-4') do
            render RubyUI::Button.new(
              variant: variant,
              size: :sm,
              href: href
            ) { button_text }
          end
        end
      end

      def render_info_row_with_edit(label, value, field)
        div(class: 'flex justify-between items-center py-3 border-b border-slate-200 last:border-0') do
          dt(class: 'text-sm font-medium text-slate-600') { label }
          div(class: 'flex items-center gap-2') do
            dd(class: 'text-sm text-slate-900') { value || 'Not set' }
            render_edit_sheet(label, field, value)
          end
        end
      end

      def render_edit_sheet(label, field, current_value, button_text: 'Edit')
        render Sheet.new do
          render SheetTrigger.new do
            render Button.new(variant: :outline, size: :sm) { button_text }
          end
          render SheetContent.new(class: 'sm:max-w-sm') do
            render SheetHeader.new do
              render(SheetTitle.new { "Edit #{label}" })
              render(SheetDescription.new { "Update your #{label.downcase} information." })
            end
            form_with(model: field == :email ? account : person, url: profile_path, method: :patch) do
              render SheetMiddle.new do
                render_field_input(field, current_value)
              end
              render SheetFooter.new do
                render Button.new(variant: :outline, data: { action: 'click->ruby-ui--sheet-content#close' }) { 'Cancel' }
                render Button.new(type: :submit) { 'Save' }
              end
            end
          end
        end
      end

      def render_field_input(field, current_value)
        case field
        when :email
          label(class: 'text-sm font-medium text-slate-900 mb-2 block') { 'Email Address' }
          render Input.new(
            type: 'email',
            name: 'account[email]',
            placeholder: 'your.email@example.com',
            value: current_value
          )
        when :date_of_birth
          label(class: 'text-sm font-medium text-slate-900 mb-2 block') { 'Date of Birth' }
          render Input.new(
            type: 'date',
            name: 'person[date_of_birth]',
            value: person.date_of_birth&.strftime('%Y-%m-%d')
          )
        end
      end

      def formatted_date_of_birth
        return 'Not set' unless person.date_of_birth

        person.date_of_birth.strftime('%B %d, %Y')
      end

      def render_email_change_sheet
        div(class: 'flex items-start justify-between p-4 border border-slate-200 rounded-lg hover:bg-slate-50 transition-colors') do
          div(class: 'flex-1') do
            h3(class: 'text-sm font-medium text-slate-900') { 'Change Email Address' }
            p(class: 'text-sm text-slate-600 mt-1') { 'Update the email address you use to sign in' }
          end
          div(class: 'ml-4') do
            render_edit_sheet('Email', :email, account.email, button_text: 'Change')
          end
        end
      end

      def render_password_change_sheet
        div(class: 'flex items-start justify-between p-4 border border-slate-200 rounded-lg hover:bg-slate-50 transition-colors') do
          div(class: 'flex-1') do
            h3(class: 'text-sm font-medium text-slate-900') { 'Change Password' }
            p(class: 'text-sm text-slate-600 mt-1') { 'Update your password to keep your account secure' }
          end
          div(class: 'ml-4') do
            render_password_sheet
          end
        end
      end

      def render_password_sheet
        render Sheet.new do
          render SheetTrigger.new do
            render Button.new(variant: :outline, size: :sm) { 'Change' }
          end
          render SheetContent.new(class: 'sm:max-w-sm') do
            render SheetHeader.new do
              render(SheetTitle.new { 'Change Password' })
              render(SheetDescription.new { 'Update your password to keep your account secure.' })
            end
            form_with(url: '/change-password', method: :post) do
              render SheetMiddle.new do
                div(class: 'space-y-4') do
                  div do
                    label(class: 'text-sm font-medium text-slate-900 mb-2 block') { 'Current Password' }
                    render Input.new(
                      type: 'password',
                      name: 'current_password',
                      placeholder: 'Enter current password',
                      required: true
                    )
                  end
                  div do
                    label(class: 'text-sm font-medium text-slate-900 mb-2 block') { 'New Password' }
                    render Input.new(
                      type: 'password',
                      name: 'new_password',
                      placeholder: 'Enter new password',
                      required: true
                    )
                  end
                  div do
                    label(class: 'text-sm font-medium text-slate-900 mb-2 block') { 'Confirm New Password' }
                    render Input.new(
                      type: 'password',
                      name: 'password_confirmation',
                      placeholder: 'Confirm new password',
                      required: true
                    )
                  end
                end
              end
              render SheetFooter.new do
                render Button.new(variant: :outline, data: { action: 'click->ruby-ui--sheet-content#close' }) { 'Cancel' }
                render Button.new(type: :submit) { 'Update Password' }
              end
            end
          end
        end
      end

      def render_close_account_dialog
        div(class: 'flex items-start justify-between p-4 border border-slate-200 rounded-lg hover:bg-slate-50 transition-colors') do
          div(class: 'flex-1') do
            h3(class: 'text-sm font-medium text-slate-900') { 'Close Account' }
            p(class: 'text-sm text-slate-600 mt-1') { 'Permanently delete your account and all associated data' }
          end
          div(class: 'ml-4') do
            render AlertDialog.new do
              render AlertDialogTrigger.new do
                render Button.new(variant: :destructive, size: :sm) { 'Close Account' }
              end
              render AlertDialogContent.new do
                render AlertDialogHeader.new do
                  render(AlertDialogTitle.new { 'Are you absolutely sure?' })
                  render(AlertDialogDescription.new do
                    'This action cannot be undone. This will permanently delete your account and remove all your data from our servers.'
                  end)
                end
                render AlertDialogFooter.new do
                  render(AlertDialogCancel.new { 'Cancel' })
                  render AlertDialogAction.new(
                    data: { turbo_method: :delete, turbo_confirm: false },
                    href: '/close-account'
                  ) { 'Yes, delete my account' }
                end
              end
            end
          end
        end
      end
    end
  end
end
