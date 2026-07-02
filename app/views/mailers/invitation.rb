# frozen_string_literal: true

module Views
  module Mailers
    class Invitation < Views::Base
      def initialize(role:, accept_url:)
        super()
        @role = role
        @accept_url = accept_url
      end

      def view_template
        render Layout.new do
          h1(class: 'mailer-title') { I18n.t('invitation_mailer.invite.title') }
          p(class: 'mailer-copy') do
            invitation_sentence
          end
          p(class: 'mailer-action-wrap') do
            a(href: @accept_url, class: 'mailer-button') { I18n.t('invitation_mailer.invite.accept_invitation') }
          end
          p(class: 'mailer-note') { I18n.t('invitation_mailer.invite.expiry_notice') }
        end
      end

      private

      def invitation_sentence
        before_role, after_role = I18n.t('invitation_mailer.invite.invited_as', role: @role).split(@role, 2)

        plain before_role
        strong { @role }
        plain after_role
      end
    end
  end
end
