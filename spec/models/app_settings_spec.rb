# frozen_string_literal: true

require "rails_helper"

RSpec.describe AppSettings do
  describe "versioning" do
    it "creates a version when invite-only mode changes" do
      settings = described_class.instance

      expect do
        settings.update!(invite_only: !settings.invite_only)
      end
        .to(change { PaperTrail::Version.where(item_type: "AppSettings", item_id: settings.id).count }.by(1))

      expect(PaperTrail::Version.where(item_type: "AppSettings", item_id: settings.id).last.event).to(eq("update"))
    end
  end
end
