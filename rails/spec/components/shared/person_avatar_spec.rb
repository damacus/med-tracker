# frozen_string_literal: true

require 'rails_helper'
require 'digest/md5'

RSpec.describe Components::Shared::PersonAvatar, type: :component do
  fixtures :accounts, :people, :users, :locations, :location_memberships

  let(:person) { people(:damacus) }

  it 'renders initials when no uploaded avatar or enabled Gravatar is present' do
    rendered = render_inline(described_class.new(person: person))

    expect(rendered.text).to include('DU')
    expect(rendered.css('img')).to be_empty
  end

  it 'does not request Gravatar until the linked account opts in' do
    rendered = render_inline(described_class.new(person: person, gravatar: true))

    expect(rendered.to_html).not_to include('gravatar.com/avatar')
  end

  it 'renders initials when Gravatar is disabled by the caller' do
    person.account.update!(gravatar_enabled: '1')

    rendered = render_inline(described_class.new(person: person, gravatar: false))

    expect(rendered.text).to include('DU')
    expect(rendered.css('img')).to be_empty
  end

  it 'renders a Gravatar image when the linked account opts in' do
    person.account.update!(gravatar_enabled: '1')
    digest = Digest::MD5.hexdigest(person.email.downcase)

    rendered = render_inline(described_class.new(person: person))

    expect(rendered.at_css("img[src*='gravatar.com/avatar/#{digest}']")).to be_present
    expect(rendered.text).to include('DU')
  end

  it 'prefers uploaded avatars over Gravatar' do
    person.account.update!(gravatar_enabled: '1')
    person.avatar.attach(io: StringIO.new('avatar'), filename: 'avatar.png', content_type: 'image/png')

    rendered = render_inline(described_class.new(person: person))

    expect(rendered.at_css("img[alt='Damacus User']")).to be_present
    expect(rendered.at_css("img[src*='/households/#{person.household.slug}/people/#{person.id}/avatar']")).to be_present
    expect(rendered.to_html).not_to include('gravatar.com/avatar')
  end

  it 'falls back when no avatar email can be resolved' do
    account = Account.create!(email: 'avatarless@example.test', status: :verified)
    avatarless_person = Person.create!(
      household: person.household,
      name: 'Avatarless Person',
      date_of_birth: 30.years.ago,
      account: account,
      email: ''
    )
    avatarless_person.update!(account: nil)

    rendered = render_inline(described_class.new(person: avatarless_person, gravatar: true))

    expect(rendered.text).to include('AP')
    expect(rendered.css('img')).to be_empty
  end

  it 'uses the linked user email when the person email is blank' do
    account = Account.create!(email: 'account-avatar@example.test', status: :verified, gravatar_enabled: '1')
    avatar_person = Person.create!(
      household: person.household,
      name: 'User Email Avatar',
      date_of_birth: 30.years.ago,
      account: account,
      email: ''
    )
    User.create!(person: avatar_person, email_address: 'user-avatar@example.test', password: 'password')
    digest = Digest::MD5.hexdigest('user-avatar@example.test')

    rendered = render_inline(described_class.new(person: avatar_person))

    expect(rendered.at_css("img[src*='gravatar.com/avatar/#{digest}']")).to be_present
  end

  it 'uses the linked account email when person and user emails are blank' do
    account = Account.create!(email: 'account-fallback@example.test', status: :verified, gravatar_enabled: '1')
    avatar_person = Person.create!(
      household: person.household,
      name: 'Account Email Avatar',
      date_of_birth: 30.years.ago,
      account: account,
      email: ''
    )
    digest = Digest::MD5.hexdigest('account-fallback@example.test')

    rendered = render_inline(described_class.new(person: avatar_person))

    expect(rendered.at_css("img[src*='gravatar.com/avatar/#{digest}']")).to be_present
  end
end
