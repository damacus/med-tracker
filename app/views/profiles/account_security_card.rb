# frozen_string_literal: true

module Views
  module Profiles
    class AccountSecurityCard < Components::Base
      include Phlex::Rails::Helpers::LinkTo

      def initialize(account:)
        @account = account
        super()
      end

      def view_template
        render Card.new(class: 'rounded-[2rem] border border-border/70 bg-card/95 shadow-[0_18px_45px_-32px_rgba(15,23,42,0.45)]') do
          render CardHeader.new do
            render(CardTitle.new { t('profiles.account_security.title') })
            render(CardDescription.new do
              t('profiles.account_security.description')
            end)
          end
          render CardContent.new(class: 'space-y-3') do
            render_email_change_sheet
            render_password_change_sheet
          end
        end
      end

      private

      def render_email_change_sheet
        div(class: 'flex items-start justify-between rounded-lg border border-border bg-card/70 p-4 transition-colors hover:bg-accent/50') do
          div(class: 'flex-1') do
            h3(class: 'text-sm font-medium text-foreground') { t('profiles.account_security.change_email_title') }
            p(class: 'mt-1 text-sm text-muted-foreground') { t('profiles.account_security.change_email_description') }
          end
          div(class: 'ml-4') do
            render RubyUI::Link.new(
              variant: :outline,
              size: :sm,
              href: '/change-login',
              data: { turbo_frame: 'modal' }
            ) { t('profiles.account_security.change_button') }
          end
        end
      end

      def render_password_change_sheet
        div(class: 'flex items-start justify-between rounded-lg border border-border bg-card/70 p-4 transition-colors hover:bg-accent/50') do
          div(class: 'flex-1') do
            h3(class: 'text-sm font-medium text-foreground') { t('profiles.account_security.change_password_title') }
            p(class: 'mt-1 text-sm text-muted-foreground') { t('profiles.account_security.change_password_description') }
          end
          div(class: 'ml-4') do
            render RubyUI::Link.new(
              variant: :outline,
              size: :sm,
              href: '/change-password',
              data: { turbo_frame: 'modal' }
            ) { t('profiles.account_security.change_button') }
          end
        end
      end
    end
  end
end
