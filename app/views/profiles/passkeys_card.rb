# frozen_string_literal: true

module Views
  module Profiles
    class PasskeysCard < Views::Base
      include Phlex::Rails::Helpers::ButtonTo
      include Phlex::Rails::Helpers::Routes

      attr_reader :account

      def initialize(account:)
        super()
        @account = account
      end

      def view_template
        render Card.new do
          render CardHeader.new do
            render(CardTitle.new { 'Passkeys' })
            render(CardDescription.new do
              'Passwordless authentication using biometrics or security keys'
            end)
          end
          render CardContent.new(class: 'space-y-4') do
            render_passkeys_list
            render_add_passkey_button
          end
        end
      end

      private

      def render_passkeys_list
        passkeys = account.account_webauthn_keys.order(created_at: :desc)

        if passkeys.empty?
          render_empty_passkeys_state
        else
          div(class: 'space-y-3') do
            passkeys.each do |passkey|
              render_passkey_item(passkey)
            end
          end
        end
      end

      def render_empty_passkeys_state
        div(class: 'text-center py-6 text-slate-500') do
          p(class: 'text-sm') { 'No passkeys registered' }
          p(class: 'text-xs mt-1') { 'Add a passkey for passwordless login' }
        end
      end

      def render_passkey_item(passkey)
        div(class: 'flex items-center justify-between p-3 border border-slate-200 rounded-lg') do
          div(class: 'flex-1') do
            p(class: 'text-sm font-medium text-slate-900') { passkey.nickname }
            p(class: 'text-xs text-slate-500') do
              "Added #{passkey.created_at.strftime('%B %d, %Y')}"
            end
          end
          button_to(
            'Remove',
            "/webauthn-remove?id=#{passkey.id}",
            method: :post,
            class: 'text-sm text-destructive hover:text-destructive/80',
            data: { turbo_confirm: 'Are you sure you want to remove this passkey?' }
          )
        end
      end

      def render_add_passkey_button
        div(class: 'pt-2') do
          render RubyUI::Link.new(
            variant: :outline,
            size: :sm,
            href: '/webauthn-setup'
          ) { 'Add a passkey' }
        end
      end
    end
  end
end
