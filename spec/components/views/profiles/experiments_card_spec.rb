# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Views::Profiles::ExperimentsCard, type: :component do
  fixtures :users

  let(:user) { users(:damacus) }

  it 'uses token-driven shells for experiments' do
    rendered = render_inline(described_class.new(user: user))
    html = rendered.to_html

    banned_classes = ['rounded-[2rem]', 'rounded-[1.2rem]', 'bg-card/95', 'bg-background/60',
                      'shadow-[0_18px_45px_-32px_rgba']

    expect(banned_classes.none? { |class_name| html.include?(class_name) }).to be(true)
  end
end
