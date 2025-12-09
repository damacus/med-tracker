# Quick Start Guide

Get MedTracker up and running in minutes with this comprehensive setup guide.

## Prerequisites

Before you begin, ensure you have the following installed:

- **Ruby** (see `.ruby-version` for the required version)
- **Bundler** - Ruby dependency manager
- **Node.js** - For JavaScript dependencies
- **PostgreSQL** - Database (or Docker to run it)

## Setup

### 1. Clone the Repository

```bash
git clone https://github.com/damacus/med-tracker.git
cd med-tracker
```

### 2. Start PostgreSQL

You have two options for running PostgreSQL:

#### Option A: Using Docker (Recommended)

```bash
docker-compose -f docker-compose.dev.yml up -d
```

#### Option B: Local PostgreSQL

If you prefer to use a local PostgreSQL installation, ensure it's running and accessible.

### 3. Install Dependencies

Install Ruby and JavaScript dependencies:

```bash
bundle install
yarn install
```

### 4. Set Up the Database

Create and migrate the database:

```bash
rails db:create
rails db:migrate
```

If you want to load sample data (optional):

```bash
rails db:seed
```

### 5. Start the Application

Start the Rails server:

```bash
bin/dev
```

Or use the traditional Rails server command:

```bash
rails server
```

The application will be available at `http://localhost:3000`.

## Environment Variables

The following environment variables can be configured for your setup:

| Variable | Description | Default |
|----------|-------------|---------|
| `DB_HOST` | PostgreSQL host | `localhost` |
| `DB_USERNAME` | PostgreSQL username | `postgres` |
| `DB_PASSWORD` | PostgreSQL password | `postgres` |
| `RAILS_ENV` | Rails environment | `development` |

To set environment variables, create a `.env` file in the project root:

```bash
cp .env.example .env
```

Then edit the `.env` file with your configuration.

## Docker Quick Start

If you prefer to run the entire application stack with Docker:

```bash
# First time setup
cp .env.example .env
docker compose -f docker-compose.dev.yml up -d
./run db:setup

# Start the app
docker compose -f docker-compose.dev.yml up
```

Access the app at `http://localhost:3000`.

For more detailed Docker deployment options, see the [Deployment Guide](deployment.md).

## Next Steps

Once you have MedTracker running:

1. **Create an Account** - Register your first user
2. **Set Up People** - Add people who will be tracked
3. **Add Medications** - Create prescriptions or add medicines
4. **Track Doses** - Log when medications are taken

## Troubleshooting

### Database Connection Issues

If you encounter database connection errors:

1. Verify PostgreSQL is running
2. Check your database credentials in `.env`
3. Ensure the database exists: `rails db:create`

### Asset Issues

If styles or JavaScript aren't loading:

1. Ensure Node.js dependencies are installed: `yarn install`
2. Restart the development server: `bin/dev`

### Port Already in Use

If port 3000 is already in use, you can specify a different port:

```bash
rails server -p 3001
```

## Running Tests

To verify your setup, run the test suite:

```bash
bundle exec rspec
```

For more information about testing, see the [Testing Guide](testing.md).

## Getting Help

- Check the [Design & Architecture](design.md) documentation
- Review the [User Management](user-management.md) guide
- Open an issue on [GitHub](https://github.com/damacus/med-tracker/issues)
