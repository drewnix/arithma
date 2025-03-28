# Multi-stage build for Arithma frontend
# Stage 1: Build Rust/WASM components
FROM rust:1.77-slim as wasm-builder
WORKDIR /app

# Install wasm-pack and dependencies
RUN apt-get update && apt-get install -y curl pkg-config libssl-dev
RUN curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Copy Rust source code
COPY Cargo.toml Cargo.lock ./
COPY src ./src/
COPY tests ./tests/

# Build the WebAssembly package
RUN wasm-pack build --target web --release

# Stage 2: Build Node.js frontend
FROM node:20-slim as frontend-builder
WORKDIR /app

# Copy package files
COPY frontend/package*.json ./
COPY frontend/tsconfig*.json ./
COPY frontend/vite.config.ts ./
COPY frontend/postcss.config.js ./
COPY frontend/tailwind.config.js ./
COPY frontend/components.json ./

# Install dependencies
RUN npm ci

# Copy frontend source
COPY frontend/src ./src
COPY frontend/public ./public
COPY frontend/index.html ./

# Copy WebAssembly build from previous stage
COPY --from=wasm-builder /app/pkg ./public/pkg/
RUN mkdir -p node_modules/arithma/ && cp -r public/pkg/* node_modules/arithma/

# Build the frontend
RUN npm run build

# Stage 3: Production with Nginx
FROM nginx:1.25-alpine
WORKDIR /usr/share/nginx/html

# Copy Nginx config
COPY --from=frontend-builder /app/dist .

# Configure Nginx for single page application
RUN echo 'server {\
    listen 80;\
    root /usr/share/nginx/html;\
    index index.html;\
    location / {\
        try_files $uri $uri/ /index.html;\
    }\
    # Cache static assets\
    location ~* \.(js|css|png|jpg|jpeg|gif|ico|svg|woff|woff2|ttf|wasm)$ {\
        expires 30d;\
        add_header Cache-Control "public, no-transform";\
    }\
    # Enable compression\
    gzip on;\
    gzip_types text/plain text/css application/json application/javascript text/xml application/xml application/xml+rss text/javascript;\
    gzip_comp_level 6;\
}' > /etc/nginx/conf.d/default.conf

# Expose port
EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]
