# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Modal, type: :component do
  subject(:rendered) do
    render_inline(described_class.new(title: 'Account security', subtitle: 'Confirm the change') do
      'Modal body'
    end)
  end

  it 'renders an open RubyUI dialog instead of the legacy native dialog wrapper' do
    expect(rendered.at_css('dialog')).to be_nil
    expect(rendered.css('[data-controller="modal"]')).to be_empty
    expect(rendered.css('[data-controller="ruby-ui--dialog"]')).to be_present
  end

  it 'preserves title, subtitle, and body content' do
    expect(rendered.at_css('h3').text).to include('Account security')
    expect(rendered.text).to include('Confirm the change')
    expect(rendered.text).to include('Modal body')
  end
end
