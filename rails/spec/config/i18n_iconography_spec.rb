# frozen_string_literal: true

require 'rails_helper'

RSpec.describe I18n do
  let(:targeted_key_paths) do
    [
      %w[medications show low_stock_alert],
      %w[person_medications card notes],
      %w[person_medications card timing_restrictions],
      %w[person_medications card next_dose_available],
      %w[person_medications card take],
      %w[person_medications card give],
      %w[person_medications card out_of_stock],
      %w[schedules card started],
      %w[schedules card ends],
      %w[schedules card notes],
      %w[schedules card next_dose_available],
      %w[schedules card take],
      %w[schedules card give],
      %w[schedules card out_of_stock]
    ]
  end
  let(:emoji_pattern) { /[\u{1F300}-\u{1FAFF}\u{2600}-\u{27BF}]/ }

  it 'keeps targeted app UI labels free of emoji in every locale' do
    locale_files.each do |locale_file|
      locale_tree = YAML.safe_load(locale_file.read).fetch(locale_file.basename('.yml').to_s)

      targeted_key_paths.each do |key_path|
        value = locale_tree.dig(*key_path)

        expect(value).to be_present
        expect(value).not_to match(emoji_pattern),
                             "#{locale_file} #{key_path.join('.')} should use SVG icons, not emoji"
      end
    end
  end

  def locale_files
    Rails.root.glob('config/locales/*.yml')
  end
end
