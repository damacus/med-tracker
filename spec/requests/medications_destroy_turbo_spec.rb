# frozen_string_literal: true

require "rails_helper"

RSpec.describe "Medications destroy with turbo_stream" do
  fixtures(:accounts, :people, :users, :locations, :medications)

  let(:admin) { users(:admin) }
  let(:medication) { medications(:vitamin_c) }

  before { sign_in(admin) }

  describe "DELETE /medications/:id" do
    it "returns turbo_stream and removes medication targets and updates flash" do
      expect do
        delete(medication_path(medication), headers: {"Accept" => "text/vnd.turbo-stream.html"})
      end
        .to(change(Medication, :count).by(-1))

      expect(response).to(have_http_status(:ok))
      expect(response.media_type).to(eq("text/vnd.turbo-stream.html"))
      expect(response.body).to(include("target=\"medication_#{medication.id}\""))
      expect(response.body).to(include("target=\"medication_show_#{medication.id}\""))
      expect(response.body).to(include("target=\"flash\""))
    end
  end
end
