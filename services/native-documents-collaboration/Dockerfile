# syntax=docker/dockerfile:1.7

FROM node:24-trixie-slim@sha256:8ec5d7557396cfe32d21c3f9c13072355ceab22b584578ca4bb28af31120cffe AS build

RUN npm install -g npm@11.21.0 && npm cache clean --force

WORKDIR /build
COPY package.json package-lock.json ./
RUN npm ci --ignore-scripts

COPY tsconfig.json tsconfig.build.json ./
COPY src ./src
RUN npm run build \
    && npm prune --omit=dev --ignore-scripts \
    && npm cache clean --force

FROM node:24-trixie-slim@sha256:8ec5d7557396cfe32d21c3f9c13072355ceab22b584578ca4bb28af31120cffe AS runtime

RUN apt-get update \
    && apt-get install --yes --no-install-recommends libpcre2-8-0 \
    && rm -rf /var/lib/apt/lists/* \
    && rm -rf /usr/local/lib/node_modules/npm /usr/local/bin/npm /usr/local/bin/npx

ENV NODE_ENV=production \
    HOME=/tmp

RUN groupadd --system --gid 10001 restless-docs \
    && useradd --system --uid 10001 --gid 10001 --no-create-home \
        --home-dir /tmp --shell /usr/sbin/nologin restless-docs

WORKDIR /opt/restless/native-documents-collaboration
COPY --from=build --chown=10001:10001 /build/package.json /build/package-lock.json ./
COPY --from=build --chown=10001:10001 /build/node_modules ./node_modules
COPY --from=build --chown=10001:10001 /build/dist ./dist

USER 10001:10001
EXPOSE 6688

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD ["node", "dist/healthcheck.js"]

CMD ["node", "dist/main.js"]
