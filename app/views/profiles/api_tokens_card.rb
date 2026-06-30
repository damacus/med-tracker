# frozen_string_literal: true

module Views
  module Profiles
    class ApiTokensCard < Components::Base
      include Phlex::Rails::Helpers::ButtonTo
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::L
      include Phlex::Rails::Helpers::Routes

      def initialize(account:, api_app_tokens:, new_api_app_token:)
        @account = account
        @api_app_tokens = api_app_tokens
        @new_api_app_token = new_api_app_token
        super()
      end

      def view_template
        m3_card(class: 'border-border/70 shadow-elevation-2', data: { testid: 'profile-api-tokens-card' }) do
          render CardHeader.new do
            render(CardTitle.new { t('profiles.api_tokens.title') })
            render(CardDescription.new { t('profiles.api_tokens.description') })
          end
          render CardContent.new(class: 'space-y-4') do
            render_new_token
            render_create_form
            render_token_list
          end
        end
      end

      private

      attr_reader :account, :api_app_tokens, :new_api_app_token

      def render_new_token
        return if new_api_app_token.blank?

        div(class: 'rounded-shape-lg border border-primary/30 bg-primary-container p-4 text-on-primary-container') do
          p(class: 'text-sm font-bold') { t('profiles.api_tokens.created_title') }
          code(class: 'mt-2 block break-all rounded-shape-md bg-surface px-3 py-2 text-xs text-foreground') do
            new_api_app_token
          end
        end
      end

      def render_create_form
        membership = account.first_active_household_membership
        return if membership.blank?

        form_with(url: profile_api_tokens_path, method: :post, class: 'space-y-3') do
          input(type: 'hidden', name: 'api_app_token[household_membership_id]', value: membership.id)
          label(class: 'block text-sm font-bold text-foreground', for: 'api_app_token_name') do
            t('profiles.api_tokens.name_label')
          end
          input(
            id: 'api_app_token_name',
            type: 'text',
            name: 'api_app_token[name]',
            maxlength: 120,
            required: true,
            class: 'block w-full rounded-shape-md border border-outline-variant bg-surface px-3 py-2 text-sm text-on-surface'
          )
          m3_button(type: :submit, variant: :filled, size: :sm) { t('profiles.api_tokens.create') }
        end
      end

      def render_token_list
        if api_app_tokens.empty?
          p(class: 'text-sm text-on-surface-variant') { t('profiles.api_tokens.empty') }
        else
          div(class: 'space-y-3') do
            api_app_tokens.each { |app_token| render_token_row(app_token) }
          end
        end
      end

      def render_token_row(app_token)
        div(class: 'rounded-shape-lg border border-border/70 bg-popover p-4 shadow-elevation-1') do
          div(class: 'flex items-start justify-between gap-4') do
            div(class: 'min-w-0') do
              h3(class: 'truncate text-sm font-medium text-foreground') { app_token.name }
              p(class: 'mt-1 text-xs text-on-surface-variant') do
                t('profiles.api_tokens.last_used_at', time: l(app_token.last_used_at, format: :short))
              end
            end
            button_to(
              t('profiles.api_tokens.revoke'),
              profile_api_token_path(app_token),
              method: :delete,
              class: 'inline-flex min-h-11 items-center justify-center rounded-shape-full border border-outline px-3 py-2 text-sm font-medium text-foreground transition-colors hover:bg-tertiary-container hover:text-on-tertiary-container'
            )
          end
        end
      end
    end
  end
end
