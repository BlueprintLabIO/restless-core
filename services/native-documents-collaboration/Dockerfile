# syntax=docker/dockerfile:1.7

FROM node:24-bookworm-slim AS build

WORKDIR /build
COPY package.json package-lock.json ./
RUN npm ci --ignore-scripts

COPY tsconfig.json tsconfig.build.json ./
COPY src ./src
RUN npm run build \
    && npm prune --omit=dev --ignore-scripts \
    && npm cache clean --force

FROM node:24-bookworm-slim AS runtime

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
