# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Views::Profiles::Show, type: :component do
  fixtures :accounts, :people, :users

  let(:user) { users(:damacus) }
  let(:account) { accounts(:damacus) }
  let(:person) { people(:damacus) }

  it 'uses token-driven shell surfaces instead of literal gradients and white overlays' do
    component = described_class.new(person: person, account: account, user: user)
    allow(component).to receive(:render_left_column) { component.send(:render_personal_info_card) }
    allow(component).to receive(:render_right_column)

    rendered = render_inline(component)
    html = rendered.to_html

    banned_classes = ['bg-[radial-gradient', 'bg-white/70', 'border-white/50', 'rounded-[2rem]',
                      'bg-card/95', 'backdrop-blur-[1.5px]']

    expect(banned_classes.none? { |class_name| html.include?(class_name) }).to be(true)
  end
end
