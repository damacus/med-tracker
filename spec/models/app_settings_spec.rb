# frozen_string_literal: true

require 'rails_helper'

RSpec.describe AppSettings do
  describe '.instance' do
    before { described_class.delete_all }

    it 'defaults invite-only mode on when an administrator already exists' do
      allow(User).to receive(:administrator).and_return(instance_double(ActiveRecord::Relation, exists?: true))

      expect(described_class.instance).to be_invite_only
    end

    it 'defaults invite-only mode off when no administrator exists' do
      allow(User).to receive(:administrator).and_return(instance_double(ActiveRecord::Relation, exists?: false))

      expect(described_class.instance).not_to be_invite_only
    end

    it 'honors an explicit INVITE_ONLY override when creating the settings row' do
      allow(User).to receive(:administrator).and_return(instance_double(ActiveRecord::Relation, exists?: true))

      original_invite_only = ENV.fetch('INVITE_ONLY', nil)
      ENV['INVITE_ONLY'] = 'false'

      expect(described_class.instance).not_to be_invite_only
    ensure
      if original_invite_only.nil?
        ENV.delete('INVITE_ONLY')
      else
        ENV['INVITE_ONLY'] = original_invite_only
      end
    end
  end

  describe 'versioning' do
    it 'creates a version when invite-only mode changes' do
      settings = described_class.instance

      expect do
        settings.update!(invite_only: !settings.invite_only)
      end.to change { PaperTrail::Version.where(item_type: 'AppSettings', item_id: settings.id).count }.by(1)

      expect(PaperTrail::Version.where(item_type: 'AppSettings', item_id: settings.id).last.event).to eq('update')
    end
  end
end
