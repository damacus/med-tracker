# frozen_string_literal: true

users_file = Rails.root.join('db/seeds/users.yml')
users_data = YAML.load_file(users_file)
household = if ENV['HOUSEHOLD_ID'].present?
              Household.find_by(id: ENV.fetch('HOUSEHOLD_ID'))
            elsif ENV['HOUSEHOLD_SLUG'].present?
              Household.find_by(slug: ENV.fetch('HOUSEHOLD_SLUG'))
            else
              Household.order(:created_at).first
            end
inviter = household&.household_memberships&.owner&.active&.order(:created_at)&.first

invited_count = 0
skipped_count = 0

if household.blank? || inviter.blank?
  Rails.logger.warn 'Skipping user invitation seeds: no household with an active owner membership was found.'
else
  users_data.each do |attrs|
    email = attrs['email'].strip
    membership_role = attrs.fetch('membership_role', 'member').strip

    if Account.exists?(email: email)
      Rails.logger.debug { "Skipping #{email} — account already exists." }
      skipped_count += 1
      next
    end

    if household.household_invitations.pending.exists?(email: email)
      Rails.logger.debug { "Skipping #{email} — pending household invitation already exists." }
      skipped_count += 1
      next
    end

    invitation = household.household_invitations.create!(
      email: email,
      membership_role: membership_role,
      invited_by_membership: inviter
    )
    InvitationMailer.with(invitation: invitation, token: invitation.plain_token).invite.deliver_later

    Rails.logger.debug { "Invited #{email} as #{membership_role} to #{household.name}." }
    invited_count += 1
  end
end

Rails.logger.debug { "User seeding complete: #{invited_count} invited, #{skipped_count} skipped." }
