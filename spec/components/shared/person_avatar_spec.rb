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

  it 'does not request Gravatar until the linked user opts in' do
    rendered = render_inline(described_class.new(person: person))

    expect(rendered.to_html).not_to include('gravatar.com/avatar')
  end

  it 'renders a Gravatar image when the linked user opts in' do
    person.user.update!(gravatar_enabled: '1')
    digest = Digest::MD5.hexdigest(person.email.downcase)

    rendered = render_inline(described_class.new(person: person))

    expect(rendered.at_css("img[src*='gravatar.com/avatar/#{digest}']")).to be_present
    expect(rendered.text).to include('DU')
  end

  it 'prefers uploaded avatars over Gravatar' do
    person.user.update!(gravatar_enabled: '1')
    person.avatar.attach(io: StringIO.new('avatar'), filename: 'avatar.png', content_type: 'image/png')

    rendered = render_inline(described_class.new(person: person))

    expect(rendered.at_css("img[alt='Damacus User']")).to be_present
    expect(rendered.to_html).not_to include('gravatar.com/avatar')
  end
end
