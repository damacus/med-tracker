# frozen_string_literal: true

require 'spec_helper'
require 'action_controller'
require 'active_support/core_ext/module/delegation'

module Components
  module Locations
    class ShowView; end
    class FormView; end
    class IndexView; end
  end
  module Layouts
    class Flash; end
  end
end

class ApplicationController < ActionController::Base
  def self.before_action(*args); end
  def authorize(*args); end
  def policy_scope(*args); end
end

require_relative '../../app/controllers/locations_controller'

RSpec.describe LocationsController do
  it 'inherits from ApplicationController' do
    expect(described_class.ancestors).to include(ApplicationController)
  end

  describe 'private methods' do
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
end
