# MedTracker Technology Stack

## Framework & Runtime

- **Ruby on Rails 8.0.1**: Modern Rails application with latest features
- **Ruby**: Version specified in `.ruby-version` file
- **SQLite3**: Database for development and testing

## Frontend Stack

- **Hotwire**: Modern Rails frontend approach
  - **Turbo Rails**: SPA-like page acceleration
  - **Stimulus**: Modest JavaScript framework for interactions
- **Importmap**: JavaScript with ESM import maps (no bundling)
- **Propshaft**: Modern asset pipeline
- **Phlex**: Component-based view layer (`phlex-rails ~> 2.3`)

## Key Dependencies

- **BCrypt**: Secure password hashing
- **Solid Stack**: Database-backed adapters
  - `solid_cache`: Rails.cache
  - `solid_queue`: Active Job
  - `solid_cable`: Action Cable
- **Bootsnap**: Boot time optimization
- **Thruster**: HTTP asset caching and compression for Puma

## Testing Framework

- **RSpec**: Primary testing framework (`rspec-rails ~> 8.0`)
- **Minitest**: Also available (Rails default)
- **Capybara + Playwright**: End-to-end browser testing
- **Shoulda Matchers**: Additional test matchers
- **Database Cleaner**: Clean test database state

## Development Tools

- **Rubocop**: Code linting and formatting
  - `rubocop-rails-omakase`: Rails styling standards
  - `rubocop-rspec`, `rubocop-capybara`: Testing-specific rules
- **Brakeman**: Security vulnerability analysis
- **Debug**: Debugging tools
- **Web Console**: Development debugging interface

## Common Commands

### Setup

```bash
bundle install          # Install Ruby dependencies
rails db:create         # Create database
rails db:migrate        # Run migrations
rails db:seed          # Seed database (if needed)
```

### Development

```bash
rails server           # Start development server (localhost:3000)
rails console         # Interactive Rails console
rails generate        # Generate Rails components
```

### Testing

```bash
bundle exec rspec      # Run RSpec tests
bundle exec rake test  # Run Minitest suite
bundle exec rubocop    # Run code linting
bundle exec brakeman   # Security analysis
```

### Database

```bash
rails db:migrate       # Run pending migrations
rails db:rollback      # Rollback last migration
rails db:reset         # Drop, create, migrate, seed
rails db:schema:load   # Load schema without running migrations
```

## Deployment

- **Kamal**: Deployment configuration present (`.kamal/` directory)
- **Docker**: Containerized deployment (`Dockerfile` present)
