# The Ask service (src/ask-main.ts) as a container – bunny.net Magic Containers (agents/CUTOVER.md 10.28). Build from the
# REPOSITORY ROOT (it needs the shared packages and a few source files the format rules are generated from):
#
#   docker build -f tools/mcp/Dockerfile -t scutoons-ask .
#   docker run --rm -p 8080:8080 -e ANTHROPIC_API_KEY=… -e SCUTOONS_API=https://api.scutoons.com/api \
#     -e ASK_ORIGINS=https://creator.scutoons.com,https://embassy.scutoons.com,https://admin.scutoons.com -e SCUTOONS_SERVICE_KEY=… scutoons-ask
#
# Only what Dockerfile.dockerignore lets through is in the build context. Nothing is compiled: Node runs the TypeScript
# sources with its type stripping, like in development.
FROM node:22-alpine

ENV NODE_ENV=production \
    ASK_PORT=8080
WORKDIR /app

# every workspace's package.json (npm ci checks them against the lock file), then only this package's dependencies
COPY package.json package-lock.json ./
COPY viewer/package.json viewer/
COPY creator/package.json creator/
COPY admin/package.json admin/
COPY shop/package.json shop/
COPY embassy/package.json embassy/
COPY shared/ shared/
COPY tools/mcp/package.json tools/mcp/
RUN npm ci --workspace @scutoons/mcp --omit=dev --ignore-scripts --no-audit --no-fund && npm cache clean --force

# the service, and the files its format rules are read from
COPY tools/mcp/ tools/mcp/
COPY creator/src/lib/editor/text-presets.ts creator/src/lib/editor/
COPY viewer/src/lib/visual-novel/README.md viewer/src/lib/visual-novel/

USER node
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=5s CMD wget -q -O - http://127.0.0.1:8080/api/ask/health || exit 1
CMD ["node", "--experimental-transform-types", "--no-warnings", "tools/mcp/src/ask-main.ts"]
