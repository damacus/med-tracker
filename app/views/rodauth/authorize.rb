module Views
  module Rodauth
    class Authorize < Views::Rodauth::Base
      def view_template
        form(method: 'post', action: rodauth.authorize_path, id: 'authorize-form',
             class: 'form-horizontal', role: 'form', data: { turbo: 'false' }) do
          authenticity_token_field
          authorization_request
          consent_scopes
          authorization_fields
          consent_actions
        end
      end

      private

      def authorization_request
        application = rodauth.oauth_application
        name = application[rodauth.oauth_applications_name_column]
        p(class: 'lead') { rodauth.authorize_page_lead(name:) }
      end

      def consent_scopes
        div(class: 'form-group') do
          h1(class: 'display-6') { rodauth.oauth_grants_scopes_label }
          rodauth.authorize_scopes.each do |scope|
            if rodauth.features.include?(:oidc) && scope == 'offline_access'
              input(type: 'hidden', name: 'scope[]', value: scope)
            else
              div(class: 'form-check') do
                input(type: 'checkbox', name: 'scope[]', value: scope, id: scope, class: 'form-check-input')
                label(for: scope, class: 'form-check-label') { scope }
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
        p(class: 'text-center') do
          input(type: 'submit', class: 'btn btn-outline-primary', value: rodauth.oauth_authorize_button)
          a(href: cancel_url, class: 'btn btn-outline-danger') { rodauth.oauth_cancel_button }
        end
      end

      def cancel_url
        params = { error: 'access_denied',
                   error_description: 'The resource owner or authorization server denied the request' }
        params[:state] = rodauth.param('state') if rodauth.param_or_nil('state')
        "#{rodauth.redirect_uri}?#{URI.encode_www_form(params)}"
      end
    end
  end
end
