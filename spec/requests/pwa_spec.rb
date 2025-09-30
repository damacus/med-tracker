# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'PWA' do
  describe 'GET /manifest.webmanifest' do
    it 'returns the web manifest with app metadata' do
      get '/manifest.webmanifest'

      expect(response).to have_http_status(:ok)
      expect(response.media_type).to eq('application/manifest+json')

      manifest = JSON.parse(response.parsed_body)
      expect(manifest['name']).to eq('MedTracker')
      expect(manifest['short_name']).to eq('MedTracker')
      expect(manifest['icons']).to contain_exactly(
        include(
          'src' => '/icons/icon-192.png',
          'sizes' => '192x192',
          'type' => 'image/png'
        ),
        include(
          'src' => '/icons/icon-512.png',
          'sizes' => '512x512',
          'type' => 'image/png'
        )
      )
    end
  end

  describe 'GET /service-worker.js' do
    it 'returns the service worker javascript' do
      get '/service-worker.js'

      expect(response).to have_http_status(:ok)
      expect(response.media_type).to eq('text/javascript')
      expect(response.body).to include('self.addEventListener')
    end
  end
end
