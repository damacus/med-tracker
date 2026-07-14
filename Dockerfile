FROM rubylang/ruby:4.0.6-resolute AS base
WORKDIR /app

ARG UID=1000
ARG GID=1000

RUN groupmod --gid "${GID}" --new-name ruby ubuntu \
  && usermod --uid "${UID}" --gid "${GID}" --login ruby --home /home/ruby --move-home ubuntu \
  && mkdir -p /usr/local/bundle \
  && chown ruby:ruby -R /app /usr/local/bundle

USER ruby

ENV GEM_HOME="/usr/local/bundle" \
  GEM_PATH="/usr/local/bundle:/usr/local/lib/ruby/gems/4.0.0" \
  PATH="${PATH}:/usr/local/bundle/bin:/home/ruby/.local/bin" \
  USER="ruby"

###############################################################################

FROM base AS gem_builder

USER root

RUN bash -c "set -o pipefail && apt-get update \
  && apt-get install -y --no-install-recommends build-essential libpq-dev libyaml-dev \
  && rm -rf /var/lib/apt/lists/* /usr/share/doc /usr/share/man \
  && apt-get clean"

USER ruby

###############################################################################

FROM gem_builder AS development

USER root

RUN apt-get update \
  && apt-get install -y --no-install-recommends curl \
  && rm -rf /var/lib/apt/lists/* /usr/share/doc /usr/share/man \
  && apt-get clean

USER ruby

ENV RAILS_ENV=development \
  NODE_ENV=development \
  BUNDLE_WITHOUT="test:production:tools" \
  BUNDLE_WITH=""

COPY --chown=ruby:ruby Gemfile* ./
RUN bundle install

COPY --chown=ruby:ruby app/ ./app/
COPY --chown=ruby:ruby bin/ ./bin/
COPY --chown=ruby:ruby config/ ./config/
COPY --chown=ruby:ruby db/ ./db/
COPY --chown=ruby:ruby lib/ ./lib/
COPY --chown=ruby:ruby public/ ./public/
COPY --chown=ruby:ruby config.ru Rakefile ./

ENTRYPOINT ["/app/bin/docker-entrypoint-web"]
CMD ["rails", "server", "-b", "0.0.0.0"]

###############################################################################

FROM gem_builder AS assets

ENV RAILS_ENV=production \
  NODE_ENV=production \
  BUNDLE_WITHOUT="development:test:tools" \
  BUNDLE_WITH="production"

COPY --chown=ruby:ruby Gemfile* ./
RUN bundle install

COPY --chown=ruby:ruby . .

RUN APP_URL=https://assets-build.invalid SECRET_KEY_BASE_DUMMY=1 rails assets:precompile

ENTRYPOINT ["/app/bin/docker-entrypoint-web"]
CMD ["rails", "server", "-b", "0.0.0.0"]

###############################################################################

FROM gem_builder AS test

USER root

RUN bash -c "set -o pipefail && apt-get update \
  && apt-get install -y --no-install-recommends postgresql-client ca-certificates curl git unzip \
    libnss3 libatk-bridge2.0-0 libdrm2 libxkbcommon0 \
    libxcomposite1 libxdamage1 libxrandr2 libgbm1 libxss1 libasound2t64 \
  && mkdir -p /etc/apt/keyrings \
  && curl -fsSL https://deb.nodesource.com/gpgkey/nodesource-repo.gpg.key -o /etc/apt/keyrings/nodesource.asc \
  && echo 'deb [signed-by=/etc/apt/keyrings/nodesource.asc] https://deb.nodesource.com/node_24.x nodistro main' > /etc/apt/sources.list.d/nodesource.list \
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

RUN rm -rf /ms-playwright \
  && npx playwright install --with-deps chromium \
  && chmod -R 755 /ms-playwright

RUN bash -c 'for browser in chromium chromium_headless_shell; do for revision in 1208 1223; do expected="/ms-playwright/${browser}-${revision}"; if [ ! -e "$expected" ]; then installed="$(find /ms-playwright -maxdepth 1 -type d -name "${browser}-*" | sort | tail -n 1)"; ln -s "$(basename "$installed")" "$expected"; fi; done; done'

USER ruby

COPY --chown=ruby:ruby app/ ./app/
COPY --chown=ruby:ruby bin/ ./bin/
COPY --chown=ruby:ruby config/ ./config/
COPY --chown=ruby:ruby db/ ./db/
COPY --chown=ruby:ruby lib/ ./lib/
COPY --chown=ruby:ruby public/ ./public/
COPY --chown=ruby:ruby spec/ ./spec/
COPY --chown=ruby:ruby .rspec .simplecov ./
COPY --chown=ruby:ruby config.ru Rakefile ./

ENTRYPOINT ["/app/bin/docker-entrypoint-web"]
CMD ["bundle", "exec", "rspec"]

###############################################################################

FROM gem_builder AS tools

USER root

RUN apt-get update \
  && apt-get install -y --no-install-recommends git \
  && rm -rf /var/lib/apt/lists/* /usr/share/doc /usr/share/man \
  && apt-get clean \
  && chown ruby:ruby -R /app

USER ruby

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

FROM base AS app

ARG APP_IMAGE_REF

USER root

RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates curl libpq5 unzip \
  && rm -rf /var/lib/apt/lists/* /usr/share/doc /usr/share/man \
  && apt-get clean \
  && chown ruby:ruby -R /app

USER ruby

RUN test -n "${APP_IMAGE_REF}" \
  && test "${APP_IMAGE_REF}" != "${APP_IMAGE_REF%:latest}:latest" \
  && printf '%s\n' "${APP_IMAGE_REF}" > /app/.runtime-image-ref \
  && chmod 0444 /app/.runtime-image-ref

RUN mkdir -p /app/storage

ENV RAILS_ENV=production \
  NODE_ENV=production \
  BUNDLE_WITHOUT="development:test:tools" \
  BUNDLE_WITH="production" \
  PATH="${PATH}:/home/ruby/.local/bin" \
  USER="ruby"

COPY --chown=ruby:ruby --from=assets /usr/local/bundle /usr/local/bundle
COPY --chown=ruby:ruby --from=assets /app/app /app/app
COPY --chown=ruby:ruby --from=assets /app/bin /app/bin
COPY --chown=ruby:ruby --from=assets /app/config /app/config
COPY --chown=ruby:ruby --from=assets /app/db /app/db
COPY --chown=ruby:ruby --from=assets /app/lib /app/lib
COPY --chown=ruby:ruby --from=assets /app/public /app/public
COPY --chown=ruby:ruby --from=assets /app/config.ru /app/Rakefile /app/
COPY --chown=ruby:ruby --from=assets /app/Gemfile /app/Gemfile.lock /app/
RUN test -f /app/public/assets/.manifest.json && ls /app/public/assets/tailwind-*.css >/dev/null 2>&1

EXPOSE 80
ENTRYPOINT ["/app/bin/docker-entrypoint-web"]
CMD ["./bin/thrust", "./bin/rails", "server"]
