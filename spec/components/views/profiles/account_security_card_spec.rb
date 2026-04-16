# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Views::Profiles::AccountSecurityCard, type: :component do
  fixtures :accounts

  let(:account) { accounts(:damacus) }

  it 'uses token-driven shells for account security' do
    rendered = render_inline(described_class.new(account: account))
    html = rendered.to_html

    banned_classes = ['rounded-[2rem]', 'bg-card/95', 'bg-card/70', 'shadow-[0_18px_45px_-32px_rgba']

    expect(banned_classes.none? { |class_name| html.include?(class_name) }).to be(true)
  end
end
