# frozen_string_literal: true

require 'rails_helper'
require 'open3'
require 'tmpdir'
require 'zlib'

RSpec.describe 'OTLP trace resource verification' do # rubocop:disable RSpec/DescribeClass
  it 'accepts plain and gzip trace bodies when every resource has host.name' do
    Dir.mktmpdir do |directory|
      plain_path = File.join(directory, 'plain.otlp')
      gzip_path = File.join(directory, 'gzip.otlp')
      write_trace_body(plain_path, resources: [resource_with_host_name])
      write_trace_body(gzip_path, resources: [resource_with_host_name], gzip: true)

      _output, error, status = verify_trace_bodies(plain_path, gzip_path)

      expect(status).to be_success, error
    end
  end

  it 'rejects a trace body when one of its resources lacks host.name' do
    Dir.mktmpdir do |directory|
      trace_path = File.join(directory, 'mixed.otlp')
      write_trace_body(trace_path, resources: [resource_with_host_name, resource_without_host_name])

      _output, error, status = verify_trace_bodies(trace_path)

      expect(status).not_to be_success
      expect(error).to include(trace_path, 'service.name')
      expect(error).not_to include('med-tracker-canary')
    end
  end

  it 'rejects a trace body without resource spans' do
    Dir.mktmpdir do |directory|
      trace_path = File.join(directory, 'empty.otlp')
      write_trace_body(trace_path, resources: [])

      _output, error, status = verify_trace_bodies(trace_path)

      expect(status).not_to be_success
      expect(error).to include(trace_path)
    end
  end

  def verify_trace_bodies(*paths)
    Open3.capture3(
      { 'OTLP_TRACE_FILES' => paths.join(':') },
      'ruby',
      Rails.root.join('scripts/verify_otlp_trace_resources.rb').to_s
    )
  end

  def write_trace_body(path, resources:, gzip: false)
    body = Opentelemetry::Proto::Collector::Trace::V1::ExportTraceServiceRequest.new(
      resource_spans: resources.map { |resource| Opentelemetry::Proto::Trace::V1::ResourceSpans.new(resource:) }
    ).to_proto
    body = Zlib.gzip(body) if gzip
    File.binwrite(path, body)
  end

  def resource_with_host_name
    resource('host.name' => 'med-tracker-canary', 'service.name' => 'med-tracker')
  end

  def resource_without_host_name
    resource('service.name' => 'med-tracker')
  end

  def resource(attributes)
    Opentelemetry::Proto::Resource::V1::Resource.new(
      attributes: attributes.map do |key, value|
        Opentelemetry::Proto::Common::V1::KeyValue.new(
          key:,
          value: Opentelemetry::Proto::Common::V1::AnyValue.new(string_value: value)
        )
      end
    )
  end
end
