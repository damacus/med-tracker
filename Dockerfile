FROM ruby:4.0.4-slim-trixie AS base
WORKDIR /app

ARG UID=1000
ARG GID=1000

RUN bash -c "set -o pipefail && apt-get update \
  && apt-get install -y --no-install-recommends build-essential curl git libpq-dev libyaml-dev unzip \
  && rm -rf /var/lib/apt/lists/* /usr/share/doc /usr/share/man \
  && apt-get clean \
  && groupadd -g \"${GID}\" ruby \
  && useradd --create-home --no-log-init -u \"${UID}\" -g \"${GID}\" ruby \
  && chown ruby:ruby -R /app /usr/local/bundle"

USER ruby

ENV PATH="${PATH}:/home/ruby/.local/bin" \
  USER="ruby"

###############################################################################

FROM base AS development

ENV RAILS_ENV=development \
  NODE_ENV=development \
  BUNDLE_WITHOUT="test:production:tools" \
  BUNDLE_WITH=""

COPY --chown=ruby:ruby Gemfile* ./
RUN bundle install

COPY --chown=ruby:ruby . .

ENTRYPOINT ["/app/bin/docker-entrypoint-web"]
CMD ["rails", "server", "-b", "0.0.0.0"]

###############################################################################

FROM base AS assets

ENV RAILS_ENV=production \
  NODE_ENV=production \
  BUNDLE_WITHOUT="development:test:tools" \
  BUNDLE_WITH="production"

COPY --chown=ruby:ruby Gemfile* ./
RUN bundle install

COPY --chown=ruby:ruby . .

RUN SECRET_KEY_BASE_DUMMY=1 rails assets:precompile

ENTRYPOINT ["/app/bin/docker-entrypoint-web"]
CMD ["rails", "server", "-b", "0.0.0.0"]

###############################################################################

FROM base AS test

USER root

RUN bash -c "set -o pipefail && apt-get update \
  && apt-get install -y --no-install-recommends postgresql-client ca-certificates curl \
    libnss3 libatk-bridge2.0-0 libdrm2 libxkbcommon0 \
    libxcomposite1 libxdamage1 libxrandr2 libgbm1 libxss1 libasound2 \
  && mkdir -p /etc/apt/keyrings \
  && curl -fsSL https://deb.nodesource.com/gpgkey/nodesource-repo.gpg.key -o /etc/apt/keyrings/nodesource.asc \
  && echo 'deb [signed-by=/etc/apt/keyrings/nodesource.asc] https://deb.nodesource.com/node_22.x nodistro main' > /etc/apt/sources.list.d/nodesource.list \
  && apt-get update && apt-get install -y --no-install-recommends nodejs \
  && rm -rf /var/lib/apt/lists/* /usr/share/doc /usr/share/man \
  && apt-get clean \
  && chown ruby:ruby -R /app"

ENV PLAYWRIGHT_BROWSERS_PATH=/ms-playwright

USER ruby

ENV RAILS_ENV=test \
  NODE_ENV=test \
  BUNDLE_WITHOUT="development:production:tools" \
  BUNDLE_WITH="" \
  PLAYWRIGHT_BROWSERS_PATH=/ms-playwright

COPY --chown=ruby:ruby Gemfile* ./
RUN bundle install

COPY --chown=ruby:ruby package.json package-lock.json ./
RUN npm ci

USER root

RUN npx playwright install --with-deps chromium \
  && chmod -R 755 /ms-playwright

USER ruby

COPY --chown=ruby:ruby . .

ENTRYPOINT ["/app/bin/docker-entrypoint-web"]
CMD ["bundle", "exec", "rspec"]

###############################################################################

FROM base AS tools

ENV RAILS_ENV=test \
  NODE_ENV=test \
  BUNDLE_WITHOUT="development:production" \
  BUNDLE_WITH="tools"

COPY --chown=ruby:ruby Gemfile* ./
RUN bundle install

COPY --chown=ruby:ruby . .

ENTRYPOINT ["/app/bin/docker-entrypoint-web"]
CMD ["bundle", "exec", "rubocop"]

###############################################################################

FROM ruby:4.0.4-slim-trixie AS app
WORKDIR /app

ARG UID=1000
ARG GID=1000

RUN apt-get update \
  && apt-get install -y --no-install-recommends curl libpq-dev unzip \
  && rm -rf /var/lib/apt/lists/* /usr/share/doc /usr/share/man \
  && apt-get clean \
  && groupadd -g "${GID}" ruby \
  && useradd --create-home --no-log-init -u "${UID}" -g "${GID}" ruby \
  && chown ruby:ruby -R /app

USER ruby

COPY --chown=ruby:ruby bin/ ./bin
RUN chmod 0755 bin/*

ENV RAILS_ENV=production \
  NODE_ENV=production \
  BUNDLE_WITHOUT="development:test:tools" \
  BUNDLE_WITH="production" \
  PATH="${PATH}:/home/ruby/.local/bin" \
  USER="ruby"

COPY --chown=ruby:ruby --from=assets /usr/local/bundle /usr/local/bundle
COPY --chown=ruby:ruby --from=assets /app/public /app/public
COPY --chown=ruby:ruby . .
RUN test -f /app/public/assets/.manifest.json && ls /app/public/assets/tailwind-*.css >/dev/null 2>&1

EXPOSE 80
ENTRYPOINT ["/app/bin/docker-entrypoint-web"]
CMD ["./bin/thrust", "./bin/rails", "server"]
