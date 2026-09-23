module Views
  module Rodauth
    class Authorize < Views::Rodauth::Base
      include Views::Rodauth::LoginBrandSupport
      include Views::Rodauth::LoginLayoutSupport

      def view_template
        login_page_layout do
          div(class: 'mx-auto flex w-full max-w-xl flex-col gap-5') do
            consent_card do
              consent_brand
              authorization_request
              authorization_form
            end
          end
        end
      end

      private

      def consent_card(&)
        div(data_authorize_surface: true,
            class: 'w-full rounded-2xl border border-outline-variant/70 bg-surface-container-lowest/95 p-5 shadow-elevation-4 backdrop-blur sm:rounded-3xl sm:p-8 dark:bg-surface-container-low/90', &)
      end

      def consent_brand
        div(data_consent_brand: true, class: 'mb-6 flex items-center justify-center gap-3 sm:justify-start') do
          render Components::Auth::MtLogo.new(label: t('app.name'))
          span(class: 'text-lg font-bold text-foreground') { t('app.name') }
        end
      end

      def authorization_request
        application = rodauth.oauth_application
        name = application[rodauth.oauth_applications_name_column]
        div(data_consent_intro: true, class: 'mb-8 space-y-2') do
          h1(class: 'text-2xl font-bold tracking-tight text-foreground sm:text-3xl') { 'Authorise access' }
          p(class: 'text-base leading-relaxed text-on-surface-variant') { rodauth.authorize_page_lead(name:) }
          consent_account_context
        end
      end

      def consent_account_context
        email_address = view_context.current_user&.email_address
        return unless email_address

        p(data_consent_account: true, class: 'text-sm font-medium text-on-surface-variant') do
          "Signed in as #{email_address}"
        end
      end

      def authorization_form
        render RubyUI::Form.new(method: :post, action: rodauth.authorize_path, id: 'authorize-form',
                                class: 'space-y-6', role: 'form', data_turbo: 'false') do
          authenticity_token_field
          consent_scopes
          authorization_fields
          consent_actions
        end
      end

      def consent_scopes
        section(data_consent_scopes: true, class: 'space-y-3', aria: { labelledby: 'consent-scopes-title' }) do
          h2(id: 'consent-scopes-title', class: 'text-lg font-bold text-foreground') { 'Access requested' }
          rodauth.authorize_scopes.each do |scope|
            if rodauth.features.include?(:oidc) && scope == 'offline_access'
              input(type: 'hidden', name: 'scope[]', value: scope)
              consent_scope_information(scope)
            else
              div(data_consent_scope_row: true, data_consent_scope: scope,
                  class: 'min-h-11 rounded-xl border border-outline-variant/70 bg-surface-container-lowest px-3 dark:bg-surface-container-low') do
                label(for: scope, class: 'flex min-h-11 cursor-pointer items-center gap-3 py-2') do
                  input(type: 'checkbox', name: 'scope[]', value: scope, id: scope,
                        class: 'size-6 shrink-0 accent-teal-700 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-teal-600 focus-visible:ring-offset-2')
                  consent_scope_text(scope)
                end
              end
            end
          end
        end
      end

      def authorization_fields
        rodauth.authorize_form_params.each do |field|
          input(type: field.fetch('type'), name: field.fetch('name'), value: field.fetch('value'))
        end
      end

      def consent_actions
        div(class: 'flex flex-col gap-3 pt-2 sm:flex-row sm:justify-end') do
          render RubyUI::Link.new(href: cancel_url, variant: :outline, size: :lg, data_consent_cancel: true,
                                  class: 'min-h-11 rounded-xl border-outline-variant px-5 font-semibold text-foreground hover:bg-surface-container-low focus-visible:ring-2 focus-visible:ring-teal-600 focus-visible:ring-offset-2') do
            rodauth.oauth_cancel_button
          end
          render RubyUI::Button.new(type: :submit, variant: :primary, size: :lg,
                                    class: 'min-h-11 cursor-pointer rounded-xl bg-teal-700 px-6 font-bold text-white shadow-sm hover:bg-teal-800 focus-visible:ring-2 focus-visible:ring-teal-600 focus-visible:ring-offset-2 dark:bg-teal-500 dark:text-slate-950 dark:hover:bg-teal-400') do
            rodauth.oauth_authorize_button
          end
        end
      end

      def consent_scope_information(scope)
        div(data_consent_scope_row: true,
            data_consent_scope: scope,
            class: 'min-h-11 rounded-xl border border-outline-variant/70 bg-surface-container-lowest px-3 py-2 dark:bg-surface-container-low') do
          consent_scope_text(scope)
        end
      end

      def consent_scope_text(scope)
        div do
          p(class: 'font-semibold text-foreground') { consent_scope_label(scope) }
          p(class: 'text-sm leading-relaxed text-on-surface-variant') { consent_scope_description(scope) }
        end
      end

      def consent_scope_label(scope)
        case scope
        when 'medtracker' then 'MedTracker data'
        when 'offline_access' then 'Stay signed in'
        else scope.humanize
        end
      end

      def consent_scope_description(scope)
        case scope
        when 'medtracker' then 'Read and update your MedTracker account data.'
        when 'offline_access' then 'Keep access active between visits.'
        else 'Access requested by this application.'
        end
      end

      def cancel_url
        params = { error: 'access_denied',
                   error_description: 'The resource owner or authorization server denied the request' }
        params[:state] = rodauth.param('state') if rodauth.param_or_nil('state')
        separator = rodauth.redirect_uri.include?('?') ? '&' : '?'
        "#{rodauth.redirect_uri}#{separator}#{URI.encode_www_form(params)}"
      end
    end
  end
end
