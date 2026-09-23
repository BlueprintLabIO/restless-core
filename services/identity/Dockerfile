# syntax=docker/dockerfile:1.7

FROM node:24-bookworm-slim AS build

WORKDIR /build
COPY package.json package-lock.json ./
RUN npm ci --omit=dev --ignore-scripts \
    && npm cache clean --force

FROM node:24-bookworm-slim AS runtime

ENV NODE_ENV=production \
    HOME=/tmp

RUN groupadd --system --gid 10001 restless-identity \
    && useradd --system --uid 10001 --gid 10001 --no-create-home \
        --home-dir /tmp --shell /usr/sbin/nologin restless-identity

# The dependency-free issuer library another host mounts (ADR 0012). Restless
# Cloud copies this directory from the image by digest.
COPY library/package.json library/issuer.d.ts /opt/restless/issuer/
COPY src/issuer.mjs /opt/restless/issuer/issuer.mjs

# The self-hosted host.
WORKDIR /opt/restless/identity
COPY --from=build /build/node_modules ./node_modules
COPY package.json package-lock.json ./
COPY src ./src
COPY public ./public
COPY scripts ./scripts

USER 10001:10001
EXPOSE 6689
CMD ["node", "src/main.mjs"]
