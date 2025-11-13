# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Admin::AuditLogs::IndexView, type: :component do
  fixtures :users, :people

  let(:admin) { users(:admin) }
  let(:person) { people(:john) }
  let(:versions) do
    PaperTrail.request.whodunnit = admin.id
    person.update!(name: 'Updated Name')
    person.update!(email: 'new@example.com')
    PaperTrail::Version.order(created_at: :desc).limit(10)
  end

  after do
    PaperTrail.request.whodunnit = nil
  end

  describe 'initialization' do
    it 'accepts versions and filter_params' do
      view = described_class.new(versions: versions, filter_params: { item_type: 'Person' })

      expect(view.versions).to eq(versions)
      expect(view.filter_params).to eq({ item_type: 'Person' })
    end

    it 'defaults filter_params to empty hash' do
      view = described_class.new(versions: versions)

      expect(view.filter_params).to eq({})
    end
  end

  describe '#filters_active?' do
    subject(:view) { described_class.new(versions: versions, filter_params: filter_params) }

    context 'when no filters are set' do
      let(:filter_params) { {} }

      it 'returns false' do
        expect(view.send(:filters_active?)).to be(false)
      end
    end

    context 'when item_type filter is set' do
      let(:filter_params) { { item_type: 'Person' } }

      it 'returns true' do
        expect(view.send(:filters_active?)).to be(true)
      end
    end

    context 'when event filter is set' do
      let(:filter_params) { { event: 'update' } }

      it 'returns true' do
        expect(view.send(:filters_active?)).to be(true)
      end
    end
  end
end
