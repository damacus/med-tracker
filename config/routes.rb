Rails.application.routes.draw do
  resource :session
  resources :passwords, param: :token
  # Defines the root path route ("/")
  root "home#index"

  namespace :admin do
    resources :users, only: [:index]
  end

  # Dashboard
  get "dashboard", to: "dashboard#index"

  # Medicine management
  resources :medicines
  get "medicine-finder", to: "medicines#finder", as: :medicine_finder

  # Authentication
  get "sign_up", to: "users#new"
  resources :users, only: %i[new create]

  get "login", to: "sessions#new"
  post "login", to: "sessions#create"
  delete "logout", to: "sessions#destroy"

  # People and prescriptions management
  resources :prescriptions do
    resources :take_medicines, only: [ :create ]
  end

  resources :people, except: [ :edit ] do
    resources :prescriptions, except: [ :index ] do
      member do
        post :take_medicine
      end
    end
  end

  # Progressive Web App assets
  get "manifest.webmanifest", to: "pwa#manifest"
  get "service-worker.js", to: "pwa#service_worker"

  # Reveal health status on /up that returns 200 if the app boots with no exceptions, otherwise 500.
  # Can be used by load balancers and uptime monitors to verify that the app is live.
  get "up" => "rails/health#show", as: :rails_health_check
end
