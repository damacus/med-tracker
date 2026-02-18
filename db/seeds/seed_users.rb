# frozen_string_literal: true

users_file = Rails.root.join('db/seeds/users.yml')
users_data = YAML.load_file(users_file)

invited_count = 0
skipped_count = 0

users_data.each do |attrs|
  email = attrs['email'].strip
  role  = attrs['role'].strip

  if Account.exists?(email: email)
    Rails.logger.debug { "Skipping #{email} — account already exists." }
    skipped_count += 1
    next
  end

  if Invitation.pending.exists?(email: email)
    Rails.logger.debug { "Skipping #{email} — pending invitation already exists." }
    skipped_count += 1
    next
  end

  invitation = Invitation.create!(email: email, role: role)
  InvitationMailer.with(invitation: invitation).invite.deliver_later

  Rails.logger.debug { "Invited #{email} as #{role}." }
  invited_count += 1
end

Rails.logger.debug { "User seeding complete: #{invited_count} invited, #{skipped_count} skipped." }
