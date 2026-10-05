# frozen_string_literal: true

class RodauthMailer < ApplicationMailer
  default to: -> { @rodauth.email_to }, from: -> { @rodauth.email_from }

  def verify_account(name, account_id, key)
    @rodauth = rodauth(name, account_id) { @verify_account_key_value = key }
    @account = @rodauth.rails_account

    mail_action_message(:verify_account, @rodauth.verify_account_email_link,
                        @rodauth.email_subject_prefix + @rodauth.verify_account_email_subject)
  end

  def reset_password(name, account_id, key)
    @rodauth = rodauth(name, account_id) { @reset_password_key_value = key }
    @account = @rodauth.rails_account
    mail_action_message(:reset_password, @rodauth.reset_password_email_link,
                        @rodauth.email_subject_prefix + @rodauth.reset_password_email_subject)
  end

  def verify_login_change(name, account_id, key)
    @rodauth = rodauth(name, account_id) { @verify_login_change_key_value = key }
    @account = @rodauth.rails_account
    mail_action_message(:verify_login_change, @rodauth.verify_login_change_email_link,
                        @rodauth.email_subject_prefix + @rodauth.verify_login_change_email_subject)
  end

  def unlock_account(name, account_id, key)
    @rodauth = rodauth(name, account_id) { @account_lockouts_key_value = key }
    @account = @rodauth.rails_account
    mail_action_message(:unlock_account, @rodauth.unlock_account_email_link,
                        @rodauth.email_subject_prefix + @rodauth.unlock_account_email_subject)
  end

  private

  def mail_action_message(action, url, subject)
    mail(subject: subject) do |format|
      format.html do
        render body: render_mail_component(action_message_component(action, url)), content_type: 'text/html'
      end
      format.text { render plain: action_message_text(action, url) }
    end
  end

  def action_message_component(action, url)
    Views::Mailers::ActionMessage.new(
      title: rodauth_mailer_translation(action, :title),
      instruction: rodauth_mailer_translation(action, :instruction),
      button_text: rodauth_mailer_translation(action, :button),
      button_url: url,
      notice: rodauth_mailer_translation(action, :notice)
    )
  end

  def action_message_text(action, url)
    [
      rodauth_mailer_translation(action, :title),
      '',
      rodauth_mailer_translation(action, :instruction),
      '',
      url,
      '',
      rodauth_mailer_translation(action, :notice)
    ].join("\n")
  end

  def rodauth_mailer_translation(action, key)
    I18n.t!("rodauth.#{action}.#{key}")
  end

  def rodauth(name, account_id, &block)
    instance = RodauthApp.rodauth(name).allocate
    instance.account_from_id(account_id)
    instance.instance_eval(&block) if block
    instance
  end
end
