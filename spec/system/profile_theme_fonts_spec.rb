# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Profile theme fonts', :js do
  fixtures :accounts, :people, :users

  it 'loads the declared Tech Indigo font in light and dark appearance' do
    login_as(users(:damacus))
    visit profile_path
    first('[data-testid="profile-appearance-sheet"] button').click
    find('button[data-theme="tech-indigo"]').click

    %w[light dark].each do |mode|
      find("button[data-appearance='#{mode}']").click
      loaded_faces = page.evaluate_async_script(<<~JS)
        const done = arguments[0];
        document.fonts.load('400 16px Geist').then(faces => {
          done(faces.map(face => ({ family: face.family, status: face.status })));
        });
      JS
      expect(loaded_faces).to include('family' => 'Geist', 'status' => 'loaded')
      expect(page.evaluate_script('getComputedStyle(document.body).fontFamily')).to eq('Geist, sans-serif')
    end
  end
end
