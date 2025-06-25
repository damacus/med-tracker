# frozen_string_literal: true

require "rails_helper"

RSpec.describe Views::Sessions::New, type: :phlex do
  # Use phlex_component helper to properly render the component
  let(:flash) { {} }
  let(:params) { {} }
  let(:component) { described_class.new(flash: flash, params: params) }
  let(:rendered) { render_inline(component).to_html }

  it "renders a form with email and password fields" do
    expect(rendered).to include('name="email_address"')
    expect(rendered).to include('name="password"')
    expect(rendered).to include('type="submit"')
    expect(rendered).to include('Sign in')
  end

  it "renders the forgot password link" do
    expect(rendered).to include('Forgot password?')
  end

  context "with flash messages" do
    let(:flash) { { alert: "Test alert message" } }

    it "renders alert messages" do
      expect(rendered).to include("Test alert message")
    end

    context "and notice messages" do
      let(:flash) { { notice: "Test notice message" } }

      it "renders notice messages" do
        expect(rendered).to include("Test notice message")
      end
    end
  end
end
