# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Login layout' do
  it 'renders without the global mobile navigation chrome' do
    get login_path

    expect(response.body).not_to include('class="nav"')
    expect(response.body).not_to include('nav__brand-link')
  end
end
