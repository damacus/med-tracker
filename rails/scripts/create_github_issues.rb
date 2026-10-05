#!/usr/bin/env ruby
# frozen_string_literal: true

require 'json'
require 'open3'

# Epic mapping: feature file basename -> epic issue number
EPIC_MAP = {
  'accessibility' => 258,
  'admin' => 259,
  'audit' => 260,
  'authentication' => 261,
  'authorization' => 262,
  'carer_relationships' => 263,
  'dashboard' => 264,
  'dosages' => 265,
  'dose_tracking' => 266,
  'e2e' => 267,
  'i18n' => 268,
  'invitations' => 269,
  'medicine_lookup' => 270,
  'medicines' => 271,
  'navigation' => 272,
  'observability' => 273,
  'people' => 274,
  'performance' => 275,
  'person_medicines' => 276,
  'prescriptions' => 277,
  'profile' => 278,
  'pwa' => 279,
  'security' => 280,
  'ui' => 281,
  'ui_improvements' => 282
}.freeze

# Label mapping based on area
LABEL_MAP = {
  'accessibility' => 'accessibility',
  'admin' => 'admin',
  'audit' => 'audit',
  'authentication' => 'authentication',
  'authorization' => 'authorization',
  'carer_relationships' => 'carer-relationships',
  'dashboard' => 'dashboard',
  'dosages' => 'dosages',
  'dose_tracking' => 'dose-tracking',
  'e2e' => 'e2e',
  'i18n' => 'i18n',
  'invitations' => 'invitations',
  'medicine_lookup' => 'medicine-lookup',
  'medicines' => 'medicines',
  'navigation' => 'navigation',
  'observability' => 'observability',
  'people' => 'people',
  'performance' => 'performance',
  'person_medicines' => 'person-medicines',
  'prescriptions' => 'prescriptions',
  'profile' => 'profile',
  'pwa' => 'pwa',
  'security' => 'security',
  'ui' => 'ui',
  'ui_improvements' => 'ui-improvements'
}.freeze

features_dir = File.expand_path('../features', __dir__)
created_count = 0

Dir.glob(File.join(features_dir, '*.json')).each do |file|
  area = File.basename(file, '.json')
  epic_num = EPIC_MAP[area]
  label = LABEL_MAP[area] || area

  next unless epic_num

  features = JSON.parse(File.read(file))
  failing = features.select { |f| f['passes'] == false }

  failing.each do |feature|
    id = feature['id']
    description = feature['description']
    steps = feature['steps'] || []

    title = "[#{id}] #{description}"
    steps_text = steps.map.with_index { |s, i| "#{i + 1}. #{s}" }.join("\n")

    body = <<~BODY
      ## Description
      #{description}

      ## Steps
      #{steps_text}

      ## Feature File
      `features/#{area}.json`

      ## Epic
      Part of ##{epic_num}
    BODY

    # Create issue via gh CLI
    cmd = [
      'gh', 'issue', 'create',
      '--repo', 'damacus/med-tracker',
      '--title', title,
      '--body', body,
      '--label', 'feature',
      '--label', label
    ]

    puts "Creating: #{title}"
    stdout, stderr, status = Open3.capture3(*cmd)

    if status.success?
      puts "  Created: #{stdout.strip}"
      created_count += 1
    else
      puts "  ERROR: #{stderr}"
    end

    # Rate limit to avoid GitHub API limits
    sleep 0.5
  end
end

puts "\nCreated #{created_count} issues"
