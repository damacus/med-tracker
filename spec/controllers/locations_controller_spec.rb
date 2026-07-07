# frozen_string_literal: true

require 'rails_helper'

RSpec.describe LocationsController do
  it 'inherits from ApplicationController' do
    expect(described_class.ancestors).to include(ApplicationController)
  end

  describe '#location_params' do
    let(:controller) { described_class.new }
    let(:params) do
      ActionController::Parameters.new(
        location: {
          name: 'Test',
          description: 'Test description',
          admin_only_field: 'secret'
        }
      )
    end

    before do
      allow(controller).to receive(:params).and_return(params)
    end

    it 'permits name and description' do
      permitted = controller.send(:location_params)
      expect(permitted.keys).to contain_exactly('name', 'description')
      expect(permitted[:name]).to eq('Test')
    end
  end
end
