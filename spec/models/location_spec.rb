# frozen_string_literal: true

require "rails_helper"

RSpec.describe Location do
  subject(:location) { described_class.new(name: "Home") }

  describe "validations" do
    it { is_expected.to(validate_presence_of(:name)) }
    it { is_expected.to(validate_uniqueness_of(:name).case_insensitive) }
  end

  describe "associations" do
    it { is_expected.to(have_many(:medications).dependent(:destroy)) }
    it { is_expected.to(have_many(:location_memberships).dependent(:destroy)) }
    it { is_expected.to(have_many(:members).through(:location_memberships).source(:person)) }
  end

  describe "versioning" do
    it "creates a version when a location changes" do
      location = create(:location)

      expect do
        location.update!(name: "Updated storage location")
      end
        .to(change { PaperTrail::Version.where(item_type: "Location", item_id: location.id).count }.by(1))

      expect(PaperTrail::Version.where(item_type: "Location", item_id: location.id).last.event).to(eq("update"))
    end
  end
end
