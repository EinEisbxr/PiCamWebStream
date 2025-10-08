# syntax=docker/dockerfile:1.6

FROM node:22-alpine AS build
WORKDIR /app

COPY package.json ./
COPY package-lock.json ./

RUN npm ci

COPY . .

ARG VITE_BACKEND_URL=http://backend:8080
ENV VITE_BACKEND_URL=${VITE_BACKEND_URL}

RUN npm run build

FROM node:22-alpine AS runtime
WORKDIR /app

COPY package.json ./
COPY package-lock.json ./
COPY vite.config.ts ./
COPY svelte.config.js ./
COPY tsconfig.json ./
COPY tsconfig.node.json ./
COPY tailwind.config.cjs ./
COPY postcss.config.cjs ./

COPY --from=build /app/node_modules ./node_modules
COPY --from=build /app/dist ./dist

ENV NODE_ENV=production

EXPOSE 4173

CMD ["npm", "run", "preview", "--", "--port", "4173"]
