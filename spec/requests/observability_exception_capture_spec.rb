# frozen_string_literal: true

require 'rails_helper'

class ObservabilityFailureController < ApplicationController
  def show
    raise 'request failure'
  end
end

RSpec.describe 'Observability exception capture' do
  around do |example|
    Rails.application.routes.draw do
      get '/observability_failure', to: 'observability_failure#show'
    end

    example.run
  ensure
    Rails.application.reload_routes!
  end

  it 'records request exceptions on the current trace span before rendering the failure response' do
    allow(Otel::ExceptionRecorder).to receive(:record)

    get '/observability_failure'

    expect(response).to have_http_status(:internal_server_error)
    expect(Otel::ExceptionRecorder).to have_received(:record)
      .with(instance_of(RuntimeError), source: 'request')
  end
end
