# frozen_string_literal: true

# Handles responses for the PWA manifest and service worker assets.
class PwaController < ApplicationController
  layout false
  allow_unauthenticated_access only: %i[manifest service_worker]
  skip_after_action :verify_pundit_authorization

  def manifest
    render body: manifest_payload.to_json
    response.headers['Content-Type'] = 'application/manifest+json'
  end

  def service_worker
    expires_in 0, public: true
    render body: service_worker_source
    response.headers['Content-Type'] = 'application/javascript'
  end

  BASE_MANIFEST_PAYLOAD = {
    name: 'MedTracker',
    short_name: 'MedTracker',
    description: 'Track medication schedules and doses anywhere.',
    start_url: '/',
    display: 'standalone',
    orientation: 'portrait',
    background_color: '#102447',
    theme_color: '#102447'
  }.freeze

  MANIFEST_ICONS = [
    {
      src: '/icons/icon-192.png',
      sizes: '192x192',
      type: 'image/png'
    },
    {
      src: '/icons/icon-512.png',
      sizes: '512x512',
      type: 'image/png'
    }
  ].freeze

  private

  def manifest_payload
    BASE_MANIFEST_PAYLOAD.merge(icons: MANIFEST_ICONS)
  end

  def service_worker_source
    Rails.public_path.join('service-worker.js').binread
  end
end
