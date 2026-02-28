# frozen_string_literal: true

# Preview all emails at http://localhost:3000/rails/mailers/rodauth_mailer
class RodauthMailerPreview < ActionMailer::Preview
  # Preview this email at http://localhost:3000/rails/mailers/rodauth_mailer/verify_account
  def verify_account
    account = Account.first || Account.new(id: 1, email: 'preview@example.com')
    RodauthMailer.verify_account(nil, account.id, 'preview-verify-key-abc123')
  end

  # Preview this email at http://localhost:3000/rails/mailers/rodauth_mailer/reset_password
  def reset_password
    account = Account.first || Account.new(id: 1, email: 'preview@example.com')
    RodauthMailer.reset_password(nil, account.id, 'preview-reset-key-abc123')
  end

  # Preview this email at http://localhost:3000/rails/mailers/rodauth_mailer/verify_login_change
  def verify_login_change
    account = Account.first || Account.new(id: 1, email: 'preview@example.com')
    RodauthMailer.verify_login_change(nil, account.id, 'preview-login-change-key-abc123')
  end

  # Preview this email at http://localhost:3000/rails/mailers/rodauth_mailer/unlock_account
  def unlock_account
    account = Account.first || Account.new(id: 1, email: 'preview@example.com')
    RodauthMailer.unlock_account(nil, account.id, 'preview-unlock-key-abc123')
  end
end
