# frozen_string_literal: true

Rails.application.routes.draw do
  # Defines the root path route ("/")
  root 'dashboard#index'

  namespace :admin do
    root to: 'dashboard#index'
    resources :users, only: %i[index new create edit update destroy] do
      member do
        post :activate
      end
    end
    resources :invitations, only: %i[index create]
    resources :carer_relationships, only: %i[index new create destroy] do
      member do
        post :activate
      end
    end
    resources :audit_logs, only: %i[index show]
  end

  get 'invitations/accept', to: 'invitations#accept', as: :accept_invitation

  # Dashboard
  get 'dashboard', to: 'dashboard#index'

  # Profile
  resource :profile, only: %i[show update]

  # Medicine management
  resources :medicines do
    member do
      get :dosages
    end
  end
  get 'medicine-finder', to: 'medicines#finder', as: :medicine_finder
  get 'medicine-finder/search', to: 'medicines#search', as: :medicine_finder_search

  # Authentication - Rodauth handles /login, /logout, /create-account via middleware

  # WebAuthn/Passkey routes are handled by Rodauth

  resources :prescriptions do
    resources :medication_takes, only: [:create]
  end

  resources :people do
    resources :prescriptions, except: [:index] do
      member do
        post :take_medicine
      end
    end

    resources :person_medicines, except: [:index] do
      member do
        post :take_medicine
      end
    end
  end

  # Progressive Web App assets
  get 'manifest.webmanifest', to: 'pwa#manifest'
  get 'service-worker.js', to: 'pwa#service_worker'

  # Reveal health status on /up that returns 200 if the app boots with no exceptions, otherwise 500.
  # Can be used by load balancers and uptime monitors to verify that the app is live.
  get 'up' => 'rails/health#show', as: :rails_health_check
end
