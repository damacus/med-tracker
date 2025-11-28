FROM ghcr.io/actions/actions-runner
# Switch to root to install dependencies
USER root

# Prevent interactive prompts during package installation
ENV DEBIAN_FRONTEND=noninteractive

# Install dependencies including .NET Core dependencies and build tools for Ruby
RUN apt-get update && apt-get install -y \
    curl \
    git \
    jq \
    sudo \
    ca-certificates \
    gnupg \
    lsb-release \
    libicu-dev \
    libssl-dev \
    libkrb5-dev \
    zlib1g-dev \
    autoconf \
    bison \
    build-essential \
    libyaml-dev \
    libreadline-dev \
    libncurses5-dev \
    libffi-dev \
    libgdbm-dev \
    gosu \
    && rm -rf /var/lib/apt/lists/*

# Install Ruby in the tool cache
ENV RUNNER_TOOL_CACHE=/opt/hostedtoolcache
RUN curl -fsSL https://github.com/rbenv/ruby-build/archive/master.tar.gz | tar xz && \
    ./ruby-build-master/install.sh && \
    rm -rf ruby-build-master && \
    ruby-build 3.4.7 /opt/hostedtoolcache/Ruby/3.4.7/arm64 && \
    touch /opt/hostedtoolcache/Ruby/3.4.7/arm64.complete

# Install Docker CLI (for Docker-in-Docker support)
RUN install -m 0755 -d /etc/apt/keyrings && \
    curl -fsSL https://download.docker.com/linux/ubuntu/gpg | gpg --dearmor -o /etc/apt/keyrings/docker.gpg && \
    chmod a+r /etc/apt/keyrings/docker.gpg && \
    echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/ubuntu \
    $(lsb_release -cs) stable" | tee /etc/apt/sources.list.d/docker.list > /dev/null && \
    apt-get update && \
    apt-get install -y docker-ce-cli docker-compose-plugin && \
    rm -rf /var/lib/apt/lists/*

# Copy entrypoint script
COPY --chown=runner:runner entrypoint.sh /home/runner/entrypoint.sh
RUN chmod +x /home/runner/entrypoint.sh

ENTRYPOINT ["/home/runner/entrypoint.sh"]
