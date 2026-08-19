# frozen_string_literal: true

require 'rails_helper'

RSpec.describe RubyUI::InputOtp, type: :component do
  let(:otp_component) do
    Class.new(Phlex::HTML) do
      def view_template
        render RubyUI::InputOtp.new(
          length: 6,
          id: 'authentication-code',
          name: 'otp',
          aria_label: 'Authentication code',
          required: true
        ) do
          render RubyUI::InputOtpGroup.new do
            6.times { |index| render RubyUI::InputOtpSlot.new(index:) }
          end
        end
      end
    end
  end
  let(:rendered) { render_inline(otp_component.new) }

  it 'renders the labelled one-time-code input contract' do
    input = rendered.at_css('input#authentication-code[name="otp"]')

    expect(input).to be_present
    expect(input['aria-label']).to eq('Authentication code')
    expect(input['autocomplete']).to eq('one-time-code')
    expect(input['inputmode']).to eq('numeric')
    expect(input['maxlength']).to eq('6')
  end

  it 'renders six presentation slots' do
    expect(rendered.css('[data-ruby-ui--input-otp-target="slot"]').count).to eq(6)
  end
end
