# frozen_string_literal: true

require "rails_helper"

RSpec.describe "Global search" do
  fixtures(
    :accounts,
    :people,
    :users,
    :locations,
    :location_memberships,
    :medications,
    :dosages,
    :schedules,
    :carer_relationships,
    :person_medications,
    :medication_takes
  )

  describe "GET /search.json" do
    it "requires authentication" do
      get(search_path(format: :json), params: {q: "Jane"})

      expect(response).to(redirect_to("/login"))
    end

    it "returns scoped JSON search results for authenticated users" do
      sign_in(users(:jane))

      get(search_path(format: :json), params: {q: "Vitamin"})

      expect(response).to(have_http_status(:ok))
      body = response.parsed_body
      expect(body.fetch("results")).to(
        include(
          hash_including("type" => "medication", "title" => "Vitamin D")
        )
      )
    end

    it "does not leak out-of-scope people through JSON search" do
      sign_in(users(:jane))

      get(search_path(format: :json), params: {q: "John"})

      expect(response).to(have_http_status(:ok))
      titles = response.parsed_body.fetch("results").pluck("title")
      expect(titles).not_to(include("John Doe"))
    end

    it "returns all matching people for administrators" do
      sign_in(users(:damacus))

      get(search_path(format: :json), params: {q: "John"})

      expect(response).to(have_http_status(:ok))
      titles = response.parsed_body.fetch("results").pluck("title")
      expect(titles).to(include("John Doe"))
    end

    it "keeps carer searches inside their assigned person scope" do
      sign_in(users(:carer))

      get(search_path(format: :json), params: {q: "John"})

      expect(response).to(have_http_status(:ok))
      titles = response.parsed_body.fetch("results").pluck("title")
      expect(titles).not_to(include("John Doe"))
    end

    it "escapes wildcard query characters" do
      sign_in(users(:damacus))

      get(search_path(format: :json), params: {q: "%"})

      expect(response).to(have_http_status(:ok))
      expect(response.parsed_body.fetch("results")).to(be_empty)
    end
  end
end
