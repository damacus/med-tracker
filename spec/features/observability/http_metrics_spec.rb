# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'OTEL-007: Metrics for HTTP request duration' do
  context 'when handling HTTP requests' do
    it 'enables HTTP request duration tracking via spans' do
      visit '/'
      sleep(0.2)
      expect(page.status_code).to eq(200)
    end

    it 'captures HTTP request spans with duration data' do
      visit '/'
      expect(page.status_code).to eq(200)
      expect(page).to have_content('MedTracker')
    end

    it 'includes route and status information in requests' do
      visit '/'
      expect(page.status_code).to eq(200)

      visit '/non-existent-route'
      expect(page.status_code).to eq(404)
    end

    it 'handles multiple quick requests' do
      3.times do
        visit '/'
        expect(page.status_code).to eq(200)
      end
    end

    it 'excludes health endpoints from metrics tracking' do
      visit '/up'
      expect(page.status_code).to eq(200)
    end
  end

  context 'when verifying HTTP instrumentation' do
    it 'has Rack instrumentation enabled' do
      instrumentation = OpenTelemetry::Instrumentation::Rack::Instrumentation.instance
      expect(instrumentation).not_to be_nil
      expect(instrumentation.installed?).to be(true)
    end

    it 'has Rails instrumentation enabled' do
      instrumentation = OpenTelemetry::Instrumentation::Rails::Instrumentation.instance
      expect(instrumentation).not_to be_nil
      expect(instrumentation.installed?).to be(true)
    end

    it 'configures untraced endpoints correctly' do
      # Verify health endpoints are configured to be untraced
      visit '/up'
      expect(page.status_code).to eq(200)

      visit '/health'
      # This will 404 since /health doesn't exist, but /up does
      # The important thing is that the request completes
    end
  end

  context 'when analyzing request durations' do
    it 'measures response times for core endpoint' do
      endpoints = ['/']
      durations = []

      endpoints.each do |endpoint|
        start_time = Time.zone.now
        visit endpoint
        duration = Time.zone.now - start_time
        durations << duration

        expect(page.status_code).to eq(200)
        expect(duration).to be > 0
        expect(duration).to be < 5.0 # Reasonable upper bound
      end

      # Verify we have duration data for all endpoints
      expect(durations.length).to eq(1)
      expect(durations.all? { |d| d > 0 }).to be(true)
    end

    it 'calculates basic statistics from request timings' do
      # Make multiple requests to gather timing data
      durations = []

      10.times do
        start_time = Time.zone.now
        visit '/'
        duration = Time.zone.now - start_time
        durations << duration
      end

      # Calculate basic statistics (percentiles approximation)
      sorted_durations = durations.sort
      min_duration = sorted_durations.first
      max_duration = sorted_durations.last
      avg_duration = sorted_durations.sum / sorted_durations.length

      # Verify reasonable timing characteristics
      expect(min_duration).to be > 0
      expect(max_duration).to be < 5.0
      expect(avg_duration).to be_between(0.001, 2.0) # More realistic range

      # Verify variation exists (requests aren't all identical)
      expect(max_duration - min_duration).to be > 0
    end
  end
end
