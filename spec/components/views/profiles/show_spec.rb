# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Views::Profiles::Show, type: :component do
  fixtures :accounts, :people, :users

  let(:account) { accounts(:damacus) }
  let(:person) { people(:damacus) }
  let(:presenter) do
    Profiles::Presenter.new(
      person:,
      account:,
      api_app_tokens: [],
      managed_notification_grants: [],
      membership: nil,
      notification_preference: person.notification_preference || person.build_notification_preference,
      passkeys: [],
      totp_enabled: false,
      recovery_codes_count: 0
    )
  end

  before do
    stub_const('Views::Profiles::CloseAccountDialog', Class.new(Components::Base) do
      def view_template
        div { 'Close account' }
      end
    end)
  end

  it 'uses token-driven shell surfaces instead of literal gradients and white overlays' do
    rendered = render_inline(described_class.new(presenter:))
    html = rendered.to_html

    banned_classes = ['bg-[radial-gradient', 'bg-white/70', 'border-white/50', 'rounded-[2rem]',
                      'bg-card/95']

    expect(banned_classes.none?(&html.method(:include?))).to be(true)
  end

  it 'renders an M3 identity header with the person avatar and profile metadata' do
    rendered = render_inline(described_class.new(presenter:))

    expect(rendered.at_css('[data-testid="profile-hero"] [data-testid="person-avatar"]')).to be_present
    expect(rendered.at_css('[data-testid="profile-avatar-sheet"]')).to be_present
    expect(rendered.text).to include('Profile photo')
    expect(rendered.text).to include('Use Gravatar')
  end

  it 'organises settings into four accessible profile sections' do
    rendered = render_inline(described_class.new(presenter:))
    triggers = rendered.css('[data-testid="profile-section-trigger"]')

    expect(triggers.map { |trigger| trigger.text.squish }).to include(
      a_string_including('Profile'),
      a_string_including('Security'),
      a_string_including('Notifications'),
      a_string_including('Advanced')
    )
    expect(triggers.find { |trigger| trigger.text.include?('Profile') }['aria-expanded']).to eq('true')
    expect(triggers.drop(1).pluck('aria-expanded')).to all(eq('false'))
    expect(rendered.at_css('[data-appearance-summary]').text).to eq('System')
    expect(triggers).to all(satisfy do |trigger|
      trigger['data-action'].include?('ruby-ui--accordion#toggle') &&
        trigger['data-ruby-ui--accordion-target'] == 'trigger'
    end)
  end

  it 'summarises password, two-factor and passkey state in Security' do
    rendered = render_inline(described_class.new(presenter:))
    security = rendered.css('[data-testid="profile-section-trigger"]').find do |trigger|
      trigger.text.include?('Security')
    end

    expect(security.text).to include('Password updated', '2FA: Not enabled', '0 passkeys')
  end

  it 'renders focused profile settings in named right-side sheets' do
    rendered = render_inline(described_class.new(presenter:))
    sheets = rendered.css('[data-controller="ruby-ui--sheet"]')

    expect(sheets.map(&:text).map(&:squish)).to include(
      a_string_including('Profile photo'),
      a_string_including('Time Zone'),
      a_string_including('Appearance')
    )
    expect(sheets.all? { |sheet| sheet.at_css('[data-ruby-ui-sheet-title]').present? }).to be(true)
    expect(sheets.all? { |sheet| sheet.at_css('template [role="dialog"]').present? }).to be(true)
  end

  it 'uses compact on and off toggle groups for binary profile settings' do
    rendered = render_inline(described_class.new(presenter:))
    groups = rendered.css('[role="radiogroup"]')

    expect(groups).not_to be_empty
    expect(groups.all? { |group| group.css('[role="radio"]').size == 2 }).to be(true)
    expect(groups.map(&:text).map(&:squish)).to all(eq('OnOff'))
  end

  it 'keeps uncommon advanced tasks in nested expandable rows' do
    rendered = render_inline(described_class.new(presenter:))
    nested = rendered.css('[data-testid="profile-nested-setting"]')

    expect(nested.pluck('aria-expanded')).to all(eq('false'))
    expect(nested.map { |trigger| trigger.text.squish }).to include(
      a_string_including('API Tokens'),
      a_string_including('Data backup'),
      a_string_including('Experiments'),
      a_string_including('System Information')
    )
  end
end
