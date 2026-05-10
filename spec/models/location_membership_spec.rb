# frozen_string_literal: true

require "rails_helper"

RSpec.describe LocationMembership do
  describe "validations" do
    subject(:membership) { build(:location_membership) }

    it { is_expected.to(validate_uniqueness_of(:person_id).scoped_to(:location_id)) }
  end

  describe "associations" do
    it { is_expected.to(belong_to(:location)) }
    it { is_expected.to(belong_to(:person)) }
  end

  describe "versioning" do
    it "creates a version when a location membership is created" do
      person = create(:person)
      location = create(:location)
      membership = nil

      expect do
        membership = create(:location_membership, person: person, location: location)
      end
        .to(change { PaperTrail::Version.where(item_type: "LocationMembership").count }.by(1))

      expect(PaperTrail::Version.where(item_type: "LocationMembership", item_id: membership.id).last.event)
        .to(eq("create"))
    end
  end
end
