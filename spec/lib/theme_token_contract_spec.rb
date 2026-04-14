# frozen_string_literal: true

require 'rails_helper'

ThemeTokenContract = Struct.new(:source)

RSpec.describe ThemeTokenContract do
  let(:source) { Rails.root.join('app/assets/tailwind/application.css').read }

  it 'authors the signed-in command centre baseline in oklch tokens' do
    [
      ':root[data-allow-palette="true"]',
      '.dark:root[data-allow-palette="true"]'
    ].each do |selector|
      block = css_block(selector)

      expect(block).to include('--palette-color: oklch(')
      expect(block).to include('--primary: oklch(')
      expect(block).to include('--background: oklch(')
      expect(block).to include('--surface-container-high: oklch(')
    end
  end

  it 'defines every profile theme seed with an oklch palette color' do
    palette_definitions = source.scan(
      /:root\[data-allow-palette="true"\]\.theme-[\w-]+\s*\{\s*--palette-color:\s*([^;]+);/m
    ).flatten

    expect(palette_definitions).not_to be_empty
    expect(palette_definitions).to all(include('oklch('))
  end

  it 'keeps derived theme overrides inside perceptual color spaces' do
    [
      ':root[data-allow-palette="true"][class*="theme-"]',
      '.dark:root[data-allow-palette="true"][class*="theme-"]'
    ].each do |selector|
      block = css_block(selector)

      expect(block).to include('color-mix(in oklab')
      expect(block).not_to match(/\bblack\b/)
      expect(block).not_to match(/\bwhite\b/)
    end
  end

  def css_block(selector)
    match = source.match(/#{Regexp.escape(selector)}\s*\{(?<body>.*?)^\}/m)

    raise "Missing CSS block for #{selector}" unless match

    match[:body]
  end
end
