# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Dosage frequency suggestions' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages

  before { sign_in(users(:admin)) }

  it 'renders frequency suggestion badges on the new dosage form' do
    medication = medications(:paracetamol)

    get new_medication_dosage_path(medication)

    expect(response).to have_http_status(:ok)
    expect(response.body).to include('Once daily')
    expect(response.body).to include('Every 4–6 hours')
    expect(response.body).to include('Every morning')
    expect(response.body).to include('As needed (PRN)')
    # FormField uses Phlex's `mix` to deep-merge `data:` hashes, so the
    # rendered attribute will be data-controller="ruby-ui--form-field frequency-suggestions"
    # (multi-controller Stimulus syntax). Assert with a regex, not a verbatim string.
    expect(response.body).to match(/data-controller="[^"]*frequency-suggestions[^"]*"/)
    expect(response.body).to include('data-action="click->frequency-suggestions#suggest"')
  end
end
