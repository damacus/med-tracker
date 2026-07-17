require 'json'
require 'open3'
require 'rails_helper'
require 'tmpdir'

RSpec.describe 'Lighthouse score gate' do
  around do |example|
    Dir.mktmpdir('lighthouse-score-gate') do |directory|
      @report_path = File.join(directory, 'lighthouse-report')
      example.run
    end
  end

  it 'passes when one low performance outlier leaves a passing median' do
    write_report(1, performance: 0.45)
    write_report(2, performance: 0.77)
    write_report(3, performance: 0.72)

    output, status = run_gate

    expect(status).to be_success
    expect(output).to include(
      'Performance:     72% (threshold: 60%)',
      'Accessibility:   95% (threshold: 90%)',
      'Best Practices:  95% (threshold: 90%)',
      'All scores meet thresholds.'
    )
  end

  it 'fails for a below-threshold median and identifies failed audits' do
    write_report(1, performance: 0.55)
    write_report(2, performance: 0.58, audits: { 'first-contentful-paint' => { score: 0.5, title: 'First Contentful Paint' } })
    write_report(3, performance: 0.77)

    output, status = run_gate

    expect(status).not_to be_success
    expect(output).to include(
      'Performance:     57% (threshold: 60%)',
      'Performance score below threshold!',
      'Top 10 failed audits:',
      '[50%] First Contentful Paint'
    )
  end

  it 'floors a boundary score consistently for display and threshold checks' do
    write_report(1, performance: 0.45)
    write_report(2, performance: 0.599)
    write_report(3, performance: 0.77)

    output, status = run_gate

    expect(status).not_to be_success
    expect(output).to include(
      'Performance:     59% (threshold: 60%)',
      'Performance score below threshold!'
    )
    expect(output).not_to include('All scores meet thresholds.')
  end

  it 'fails closed when an expected report is missing' do
    write_report(1, performance: 0.72)
    write_report(3, performance: 0.77)

    output, status = run_gate

    expect(status).not_to be_success
    expect(output).to include("Missing Lighthouse JSON report: #{@report_path}-2.report.json")
  end

  it 'fails closed when an expected report is malformed' do
    write_report(1, performance: 0.72)
    File.write("#{@report_path}-2.report.json", '{')
    write_report(3, performance: 0.77)

    output, status = run_gate

    expect(status).not_to be_success
    expect(output).to include("Malformed Lighthouse JSON report: #{@report_path}-2.report.json")
  end

  def run_gate
    stdout, stderr, status = Open3.capture3(
      'node',
      Rails.root.join('bin/lighthouse_score_gate.js').to_s,
      '--report-path', @report_path,
      '--runs', '3',
      '--perf-threshold', '60',
      '--a11y-threshold', '90',
      '--bp-threshold', '90'
    )

    [stdout + stderr, status]
  end

  def write_report(run, performance:, accessibility: 0.95, best_practices: 0.95, audits: {})
    report = {
      categories: {
        performance: { score: performance },
        accessibility: { score: accessibility },
        'best-practices': { score: best_practices }
      },
      audits: audits
    }

    File.write("#{@report_path}-#{run}.report.json", JSON.generate(report))
  end
end
