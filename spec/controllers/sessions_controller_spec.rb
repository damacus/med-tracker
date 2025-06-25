# frozen_string_literal: true

require "rails_helper"

RSpec.describe SessionsController, type: :controller do
  describe "GET #new" do
    it "renders the Phlex view" do
      get :new
      expect(response).to have_http_status(:ok)
      expect(response.body).to include("Email address")
      expect(response.body).to include("Password")
      expect(response.body).to include("Sign in")
    end

    it "passes flash messages to the view" do
      flash[:alert] = "Test alert"
      get :new
      expect(response.body).to include("Test alert")
    end
  end
end
